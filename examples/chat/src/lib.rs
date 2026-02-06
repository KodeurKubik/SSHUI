use sshui::{
    InputEvent, KeyEvent, SSHUITerminal,
    ratatui::{
        Frame,
        layout::{Constraint, Flex, Layout, Margin},
        prelude::Alignment,
        style::Stylize,
        symbols::border,
        text::Line,
        widgets::{Block, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    },
};

#[derive(Clone, PartialEq)]
pub struct Message {
    pub is_self: bool,
    pub author: String,
    pub content: String,
}

pub struct App {
    entering_username: bool,
    username: String,
    typing: String,
    messages: Vec<Message>,
    just_sent: Option<Message>,
    lobby: sshui::Lobby<Message>,
    rx: tokio::sync::broadcast::Receiver<Message>,
    scroll: usize,
    last_max_scroll: usize,
    scroll_state: ScrollbarState,
    exit: bool,
}

impl sshui::App for App {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> anyhow::Result<Option<String>> {
        terminal.draw(|frame| self.draw(frame))?;
        Ok(self.exit.then(|| "Exited".to_string()))
    }

    fn input(&mut self, event: InputEvent) {
        let InputEvent::Key(KeyEvent { key, .. }) = event else {
            return;
        };

        if self.entering_username {
            match key {
                sshui::KeyCode::Char(c) => {
                    if self.username.len() < 20 {
                        self.username.push(c);
                    }
                }
                sshui::KeyCode::Backspace => {
                    self.username.pop();
                }
                sshui::KeyCode::Enter => {
                    if !self.username.trim().is_empty() {
                        self.entering_username = false;
                    }
                }
                sshui::KeyCode::Escape => self.exit = true,
                _ => {}
            }
            return;
        }

        match key {
            sshui::KeyCode::Char(c) => {
                if self.typing.len() < 200 {
                    self.typing.push(c)
                }
            }
            sshui::KeyCode::Backspace => {
                self.typing.pop();
            }
            sshui::KeyCode::UpArrow => {
                self.scroll = self.scroll.saturating_sub(1);
                self.scroll_state = self.scroll_state.position(self.scroll);
            }
            sshui::KeyCode::DownArrow => {
                self.scroll = self.scroll.saturating_add(1);
                self.scroll_state = self.scroll_state.position(self.scroll);
            }
            sshui::KeyCode::Enter => self.send_message(),
            sshui::KeyCode::Escape => self.exit = true,
            _ => {}
        }
    }
}

impl App {
    pub fn new(lobby: sshui::Lobby<Message>) -> Self {
        let rx = lobby.subscribe();

        Self {
            entering_username: true,
            username: String::new(),
            typing: String::new(),
            messages: Vec::new(),
            just_sent: None,
            lobby,
            rx,
            scroll: usize::MAX,
            last_max_scroll: 0,
            scroll_state: ScrollbarState::default(),
            exit: false,
        }
    }

    fn send_message(&mut self) {
        let typed = std::mem::take(&mut self.typing);
        if typed.trim().is_empty() {
            return;
        }

        let mut to_send = Message {
            is_self: false,
            author: self.username.clone(),
            content: typed.clone(),
        };

        self.just_sent = Some(to_send.clone());
        self.lobby.send(to_send.clone());

        to_send.is_self = true;
        self.messages.push(to_send);

        self.scroll = usize::MAX;
    }

    fn build_lines(messages: &[Message], width: usize) -> Vec<Line<'_>> {
        let mut lines: Vec<Line> = Vec::new();

        for (i, msg) in messages.iter().enumerate() {
            let prefix = format!("{}: ", msg.author);
            let indent = prefix.chars().count();
            let content_w = width.saturating_sub(indent);
            let align = if msg.is_self {
                Alignment::Right
            } else {
                Alignment::Left
            };

            if content_w == 0 {
                lines.push(Line::from(format!(" {prefix}{} ", msg.content)).alignment(align));
            } else {
                let mut cur = String::new();
                let mut first = true;

                let push_line = |cur: &mut String, first: &mut bool, lines: &mut Vec<Line>| {
                    if cur.is_empty() {
                        return;
                    }
                    let pfx = if *first {
                        *first = false;
                        prefix.clone()
                    } else {
                        " ".repeat(indent)
                    };
                    lines.push(Line::from(format!(" {pfx}{cur} ")).alignment(align));
                    cur.clear();
                };

                for word in msg.content.split_whitespace() {
                    let wlen = word.chars().count();

                    if wlen > content_w {
                        push_line(&mut cur, &mut first, &mut lines);
                        for chunk in word.chars().collect::<Vec<_>>().chunks(content_w) {
                            let pfx = if first {
                                first = false;
                                prefix.clone()
                            } else {
                                " ".repeat(indent)
                            };
                            let s: String = chunk.iter().collect();
                            lines.push(Line::from(format!(" {pfx}{s} ")).alignment(align));
                        }
                    } else if cur.is_empty() {
                        cur = word.to_string();
                    } else if cur.chars().count() + 1 + wlen <= content_w {
                        cur.push(' ');
                        cur.push_str(word);
                    } else {
                        push_line(&mut cur, &mut first, &mut lines);
                        cur = word.to_string();
                    }
                }

                if !cur.is_empty() || first {
                    let pfx = if first { &prefix } else { &" ".repeat(indent) };
                    lines.push(Line::from(format!(" {pfx}{cur} ")).alignment(align));
                }
            }

            if i + 1 < messages.len() {
                lines.push(Line::from(""));
            }
        }

        lines
    }

    fn draw(&mut self, frame: &mut Frame) {
        let was_at_bottom = self.scroll == usize::MAX || self.scroll >= self.last_max_scroll;

        while let Ok(msg) = self.rx.try_recv() {
            if let Some(sent) = &self.just_sent {
                if &msg == sent {
                    self.just_sent = None;
                    continue;
                }
            }

            if was_at_bottom {
                self.scroll = usize::MAX;
            }
            self.messages.push(msg);
        }

        let area = frame.area();
        let inner_w = area.width.saturating_sub(2) as usize;
        let input_lines = if inner_w == 0 {
            1
        } else {
            (self.typing.chars().count().div_ceil(inner_w) as u16).max(1)
        };

        let [chat_area, input_area] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(input_lines + 2)]).areas(area);

        let chat_block = Block::bordered()
            .title(" Chat ".bold())
            .border_set(border::ROUNDED);
        let inner = chat_block.inner(chat_area);
        let visible = inner.height as usize;

        let lines = Self::build_lines(&self.messages, inner.width as usize);
        let max_scroll = lines.len().saturating_sub(visible);

        self.scroll = self.scroll.min(max_scroll);
        self.last_max_scroll = max_scroll;
        self.scroll_state = self
            .scroll_state
            .content_length(max_scroll)
            .position(self.scroll);

        frame.render_widget(
            Paragraph::new(lines)
                .block(chat_block)
                .scroll((self.scroll as u16, 0)),
            chat_area,
        );
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            chat_area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scroll_state,
        );

        frame.render_widget(
            Paragraph::new(self.typing.as_str())
                .wrap(Wrap { trim: false })
                .block(
                    Block::bordered()
                        .title(format!(" Message (as {}) ", self.username).bold())
                        .border_set(border::ROUNDED),
                ),
            input_area,
        );

        if self.entering_username {
            let popup_w = 40u16.min(area.width.saturating_sub(4));
            let popup_h = 3u16;
            let [popup_area] = Layout::horizontal([Constraint::Length(popup_w)])
                .flex(Flex::Center)
                .areas(
                    Layout::vertical([Constraint::Length(popup_h)])
                        .flex(Flex::Center)
                        .areas::<1>(area)[0],
                );
            frame.render_widget(Clear, popup_area);
            frame.render_widget(
                Paragraph::new(self.username.as_str()).block(
                    Block::bordered()
                        .title(" Username ".bold())
                        .border_set(border::ROUNDED),
                ),
                popup_area,
            );
        }
    }
}
