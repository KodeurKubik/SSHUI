use sshui::{
    InputEvent, KeyEvent, SSHUITerminal,
    ratatui::{
        Frame,
        buffer::Buffer,
        layout::{Alignment, Constraint, Rect},
        symbols::border,
        text::{Line, Text},
        widgets::{Block, Paragraph, Widget},
    },
};

const FRAMES: &str = include_str!("./badapple.ascii");
const WIDTH: usize = 96;

pub struct App {
    exit: bool,
    frames: Vec<Vec<&'static str>>,
    frame_index: usize,
}

impl sshui::App for App {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> anyhow::Result<Option<String>> {
        if self.frame_index > self.frames.len() {
            return Ok(Some("Exited".to_string()));
        }

        terminal.draw(|frame| self.draw(frame))?;

        Ok(if self.exit {
            Some("Exited".to_string())
        } else {
            None
        })
    }

    fn input(&mut self, event: sshui::InputEvent) {
        let InputEvent::Key(KeyEvent { key, .. }) = event else {
            return;
        };

        match key {
            sshui::KeyCode::Char('q') | sshui::KeyCode::Escape => self.exit = true,

            _ => {}
        }
    }

    fn on_tick(&mut self) {
        self.frame_index += 1;
    }
}

impl App {
    pub fn default() -> Self {
        let frames: Vec<Vec<&'static str>> = FRAMES
            .lines()
            .map(|line| {
                let chars: Vec<char> = line.chars().collect();
                let mut slices = Vec::new();
                let mut start = 0;
                for chunk in chars.chunks(WIDTH) {
                    let end = start + chunk.iter().map(|c| c.len_utf8()).sum::<usize>();
                    slices.push(&line[start..end]);
                    start = end;
                }
                slices
            })
            .collect();

        Self {
            exit: false,
            frames,
            frame_index: 0,
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .border_set(border::PLAIN)
            .title(" BAD APPLE ")
            .title_alignment(Alignment::Center);
        let inner = block.inner(area);
        block.render(area, buf);

        if self.frame_index > 0 && self.frame_index <= self.frames.len() {
            let current = &self.frames[self.frame_index - 1];
            let content_height = current.len() as u16;
            let centered_area = inner.centered_vertically(Constraint::Length(content_height));

            Paragraph::new(Text::from(
                current
                    .iter()
                    .map(|&s| Line::from(s).centered())
                    .collect::<Vec<_>>(),
            ))
            .centered()
            .render(centered_area, buf);
        }
    }
}
