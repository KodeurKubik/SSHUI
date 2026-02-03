use anyhow::Result;
use sshui::ratatui::layout::{Constraint, Direction, Layout, Margin};
use sshui::ratatui::style::Style;
use sshui::ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use sshui::ratatui::{
    buffer::Buffer, layout::Rect, prelude::Widget, style::Stylize, symbols::border, text::Line,
    widgets::Block,
};
use sshui::{InputEvent, KeyCode, KeyEvent, SSHUITerminal};

pub const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(66);

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    let key_pair = sshui::get_debug_ssh_key()?;
    #[cfg(not(debug_assertions))]
    let key_pair = sshui::get_ssh_key()?;

    let config = sshui::Config {
        keys: vec![key_pair],
        ..Default::default()
    };

    let mut port = 2222u16;
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            port = args[i + 1].parse().unwrap_or(2222);
            break;
        }
    }

    sshui::new_server_with_refresh(config, ("0.0.0.0", port), TICK_RATE, || {
        Box::new(ExampleApp {
            selected: 0,
            app: None,
            exit_message: None,
            vertical_scroll: 0,
            vertical_scroll_state: ScrollbarState::default(),
        })
    })
    .await?;

    Ok(())
}

pub struct ExampleApp {
    selected: usize,
    app: Option<Box<dyn sshui::App>>,
    exit_message: Option<String>,
    vertical_scroll: usize,
    vertical_scroll_state: ScrollbarState,
}

impl sshui::App for ExampleApp {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
        if let Some(app) = &mut self.app {
            return app.render(terminal);
        }

        terminal.draw(|frame| {
            self.render_widget(frame.area(), frame.buffer_mut());
        })?;

        if self.exit_message.is_some() {
            Ok(self.exit_message.clone())
        } else {
            Ok(None)
        }
    }

    fn input(&mut self, event: InputEvent) {
        if let Some(app) = &mut self.app {
            app.input(event);
            return;
        }

        let InputEvent::Key(KeyEvent { key, .. }) = event else {
            return;
        };

        match key {
            KeyCode::Enter => {
                self.app = Some(get_projects().into_iter().nth(self.selected).unwrap().app);
            }
            KeyCode::DownArrow => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::UpArrow => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.vertical_scroll_state =
                    self.vertical_scroll_state.position(self.vertical_scroll);
            }
            KeyCode::RightArrow => {
                if self.selected + 1 < get_projects().len() {
                    self.selected += 1;
                }
            }
            KeyCode::LeftArrow => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Escape => {
                self.exit_message = Some("Thank you for testing the SSHUI demo!".to_string())
            }
            _ => {}
        }
    }

    fn on_tick(&mut self) {
        if let Some(app) = &mut self.app {
            return app.on_tick();
        }
    }
}

impl ExampleApp {
    fn render_widget(&mut self, area: Rect, buf: &mut Buffer) {
        let instructions = Line::from(vec![
            " Use ".into(),
            "<arrows>".blue().bold(),
            " to select and ".into(),
            "<ENTER>".blue().bold(),
            " to run. Press ".into(),
            "<ESC>".blue().bold(),
            " to leave ".into(),
        ]);
        let block = Block::bordered()
            .title(Line::from(" S S H U I  Demo ".bold()).centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);
        block.render(area, buf);

        let inner = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let projects = get_projects();
        let proj_len = (projects.len() as f32 / 2.0).ceil() as usize;
        let row_height: u16 = 8;
        let total_height = proj_len as u16 * row_height;
        let max_scroll = total_height.saturating_sub(inner.height) as usize;

        self.vertical_scroll = self.vertical_scroll.min(max_scroll);
        self.vertical_scroll_state = self
            .vertical_scroll_state
            .content_length(max_scroll)
            .position(self.vertical_scroll);

        for i in 0..proj_len {
            let row_y =
                inner.y as i32 + (i as i32 * row_height as i32) - self.vertical_scroll as i32;
            let row_bottom = row_y + row_height as i32;

            if row_bottom <= inner.y as i32 || row_y >= (inner.y + inner.height) as i32 {
                continue;
            }

            let clip_top = (inner.y as i32 - row_y).max(0) as u16;
            let clip_bottom = (row_bottom - (inner.y + inner.height) as i32).max(0) as u16;
            let visible_y = row_y.max(inner.y as i32) as u16;
            let visible_height = row_height - clip_top - clip_bottom;

            let row_rect = Rect {
                x: inner.x,
                y: visible_y,
                width: inner.width,
                height: visible_height,
            };

            let boxes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(2),
                    Constraint::Fill(1),
                    Constraint::Length(2),
                ])
                .split(row_rect);

            let proj1 = &projects[2 * i];
            let box1 = if 2 * i == self.selected {
                Block::bordered()
                    .border_set(border::ROUNDED)
                    .border_style(Style::new().green())
            } else {
                Block::bordered().border_set(border::ROUNDED)
            };
            let content1 = Paragraph::new(vec![
                Line::from(proj1.title.to_string().bold()),
                Line::from(""),
                Line::from(proj1.description.to_string()),
            ])
            .centered();
            content1.block(box1).render(boxes[0], buf);

            if projects.len() > 2 * i + 1 {
                let proj2 = &projects[2 * i + 1];
                let box2 = if 2 * i + 1 == self.selected {
                    Block::bordered()
                        .border_set(border::ROUNDED)
                        .border_style(Style::new().green())
                } else {
                    Block::bordered().border_set(border::ROUNDED)
                };
                let content2 = Paragraph::new(vec![
                    Line::from(proj2.title.to_string().bold()),
                    Line::from(""),
                    Line::from(proj2.description.to_string()),
                ])
                .centered();
                content2.block(box2).render(boxes[2], buf);
            }
        }

        sshui::ratatui::prelude::StatefulWidget::render(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            inner,
            buf,
            &mut self.vertical_scroll_state,
        );
    }
}

struct ProjectStruct {
    title: String,
    description: String,
    app: Box<dyn sshui::App>,
}

fn get_projects() -> Vec<ProjectStruct> {
    vec![
        ProjectStruct {
            title: "Demo".to_string(),
            description: "show everything sshui x ratatui can do!".to_string(),
            app: Box::new(demo_ssh::App::new(demo_ssh::ENHANCED_GRAPHICS)),
        },
        ProjectStruct {
            title: "Grapher".to_string(),
            description: "Graph functions in the terminal! (holy moly)".to_string(),
            app: Box::new(graph_ssh::GraphApp::default()),
        },
        ProjectStruct {
            title: "Bad Apple".to_string(),
            description: "Watch bad apple.... in the terminal??".to_string(),
            app: Box::new(badapple_ssh::App::default()),
        },
        ProjectStruct {
            title: "Wordle".to_string(),
            description: "just a regular game of worlde lol".to_string(),
            app: Box::new(wordle_ssh::WordleApp::default()),
        },
        ProjectStruct {
            title: "Counter".to_string(),
            description: "that's litteraly just like a hello world".to_string(),
            app: Box::new(counter_ssh::App::default()),
        },
    ]
}

// TODO? typing test: calculate your words per minute by typing (fast)!
