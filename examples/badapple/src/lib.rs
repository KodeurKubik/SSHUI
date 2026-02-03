use sshui::{
    InputEvent, KeyEvent, SSHUITerminal,
    ratatui::{
        Frame,
        buffer::Buffer,
        layout::Rect,
        text::{Line, Text},
        widgets::{Block, Clear, Paragraph, Widget},
    },
};

const FRAMES: &str = include_str!("./badapple.ascii");
const WIDTH: usize = 96;
const HEIGHT: usize = 36;

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
            sshui::KeyCode::Char(char) => {
                if char == 'q' {
                    self.exit = true
                }
            }
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
        Clear.render(area, buf);

        const BOX_WIDTH: u16 = WIDTH as u16 + 2;
        const BOX_HEIGHT: u16 = HEIGHT as u16 + 2;

        let box_area = Rect {
            x: area.x + area.width.saturating_sub(BOX_WIDTH) / 2,
            y: area.y + area.height.saturating_sub(BOX_HEIGHT) / 2,
            width: BOX_WIDTH,
            height: BOX_HEIGHT,
        };

        let block = Block::bordered().title(" BAD APPLE ");
        let inner = block.inner(box_area);
        block.render(box_area, buf);

        if self.frame_index > 0 && self.frame_index <= self.frames.len() {
            let current = &self.frames[self.frame_index - 1];
            Paragraph::new(Text::from(
                current
                    .iter()
                    .map(|&s| {
                        let char_count = s.chars().count();
                        if char_count < WIDTH {
                            Line::from(format!("{s:<WIDTH$}"))
                        } else {
                            Line::from(s)
                        }
                    })
                    .collect::<Vec<_>>(),
            ))
            .render(inner, buf);
        }
    }
}
