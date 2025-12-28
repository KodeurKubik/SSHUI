// Ported version of Ratatui's Basic Counter App
// Available under the MIT License at: https://github.com/ratatui/ratatui-website/blob/main/code/tutorials/counter-app-basic/src/main.rs

use std::io;

use anyhow::Result;
// use ratatui from sshui
use sshui::ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};
use sshui::{self, InputEvent, KeyCode, KeyEvent, SSHUITerminal};

// This function changes quite a lot
// instead of running the terminal directly,
// run the server with a on-client-accept closure
// and it has to go async
#[tokio::main]
async fn main() -> io::Result<()> {
    let key_pair = sshui::get_ssh_key().unwrap();
    let config = sshui::Config {
        keys: vec![key_pair],
        ..Default::default()
    };

    sshui::new_server(config, ("0.0.0.0", 2222), || Box::new(App::default()))
        .await
        .unwrap();

    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    counter: u8,
    exit: bool,
}

// add this `impl for sshui::App` to support the new_server
impl sshui::App for App {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
        // run here instead of App::run, but no while or handle_events
        terminal.draw(|frame| self.draw(frame))?;

        Ok(if self.exit {
            Some("Exited".to_string())
        } else {
            None
        })
    }

    fn input(&mut self, event: InputEvent) {
        let InputEvent::Key(KeyEvent { key, .. }) = event else {
            return;
        };

        match key {
            KeyCode::Char('q') => self.exit(),
            KeyCode::LeftArrow => self.decrement_counter(),
            KeyCode::RightArrow => self.increment_counter(),
            _ => {}
        }
    }
}

impl App {
    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    // We can remove these function now!
    // pub fn run
    // fn handle_events
    // fn handle_key_event

    fn exit(&mut self) {
        self.exit = true;
    }

    // just made it overflow and underflow support
    fn increment_counter(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }

    fn decrement_counter(&mut self) {
        self.counter = self.counter.wrapping_sub(1);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = Text::from(vec![Line::from(vec![
            "Value: ".into(),
            self.counter.to_string().yellow(),
        ])]);

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
