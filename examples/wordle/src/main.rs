// Ported version of my Wordle
// Available under the MIT License at: https://github.com/KodeurKubik/wordle-rust

mod wordlist;
use crate::wordlist::{VALIDLIST, WORDLIST};
use anyhow::Result;
use rand::seq::IndexedRandom;
use sshui::ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph},
};
use sshui::{InputEvent, KeyCode, KeyEvent, SSHUITerminal};
use std::collections::HashMap;

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

    sshui::new_server(config, ("0.0.0.0", 2222), || {
        let mut rng = rand::rng();
        let word: Vec<char> = WORDLIST.choose(&mut rng).unwrap().chars().collect();

        Box::new(WordleApp {
            correct: word.try_into().unwrap(),
            guesses: Vec::with_capacity(6),
            typing: [None; 5],
            message: Line::from(""),
            exit_message: None,
        })
    })
    .await?;

    Ok(())
}

#[derive(Debug, Default)]
pub struct WordleApp {
    correct: [char; 5],
    guesses: Vec<[char; 5]>,
    typing: [Option<char>; 5],
    message: Line<'static>,
    exit_message: Option<String>,
}

impl sshui::App for WordleApp {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
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
        let InputEvent::Key(KeyEvent { key, .. }) = event else {
            return;
        };

        match key {
            KeyCode::Escape => {
                self.exit_message = Some(format!(
                    "The word was {} but you gave up. >:(",
                    self.correct.iter().collect::<String>()
                ));
            }
            KeyCode::Backspace => {
                self.message = Line::from("");
                self.typing.reverse();
                for k in self.typing.iter_mut() {
                    if k.is_some() {
                        *k = None;
                        break;
                    }
                }
                self.typing.reverse();
            }
            KeyCode::Enter => {
                if self.typing.contains(&None) {
                    self.message = Line::from("5 chars needed!".red());
                    return;
                }

                let word: String = self.typing.iter().filter_map(|c| c.as_ref()).collect();

                if !VALIDLIST.contains(&word.as_str()) {
                    self.message = Line::from("not a valid word!".red());
                    return;
                }

                let correct = self.correct.iter().collect::<String>();
                if word == correct {
                    self.exit_message = Some(format!(
                        "Congratz! The word was indeed {}. You guessed it in {} tries.",
                        correct,
                        self.guesses.len() + 1
                    ));
                    self.message = Line::from("you won!!".green());
                    return;
                }

                self.message = Line::from("errr! try again".red());
                let guess: [char; 5] = word.chars().collect::<Vec<_>>().try_into().unwrap();
                self.guesses.push(guess);
                self.typing = [None; 5];

                if self.guesses.len() >= 6 {
                    self.exit_message =
                        Some(format!("You lost after 6 tries! The word was {}.", correct));
                }
            }
            KeyCode::Char(c) if c.is_ascii_alphabetic() => {
                self.message = Line::from("");
                let smol = c.to_ascii_uppercase();

                for k in self.typing.iter_mut() {
                    if k.is_none() {
                        *k = Some(smol);
                        break;
                    }
                }

                if !self.typing.contains(&None) {
                    let word: String = self.typing.iter().filter_map(|c| c.as_ref()).collect();
                    if !VALIDLIST.contains(&word.as_str()) {
                        self.message = Line::from("not a valid word!".red());
                    }
                }
            }
            _ => {}
        }
    }
}

impl WordleApp {
    fn render_widget(&self, area: Rect, buf: &mut Buffer) {
        #[cfg(debug_assertions)]
        let title = Line::from(vec![
            " W O R D L E ".bold(),
            "- ".into(),
            self.correct.iter().collect::<String>().into(),
            " ".into(),
        ]);

        #[cfg(not(debug_assertions))]
        let title = Line::from(" W O R D L E ".bold());

        let instructions = Line::from(vec![
            " Press a ".into(),
            "<key>".blue().bold(),
            " Confirm ".into(),
            "<ENTER>".blue().bold(),
            " Quit ".into(),
            "<ESC> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner = block.inner(area);
        block.render(area, buf);

        let cell_width: u16 = 7;
        let row_height: u16 = 3;
        let total_cells_width: u16 = cell_width * 5;

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Max(1),
                Constraint::Length(row_height),
                Constraint::Length(row_height),
                Constraint::Length(row_height),
                Constraint::Length(row_height),
                Constraint::Length(row_height),
                Constraint::Length(row_height),
            ])
            .split(inner);

        let message_block = Block::default();
        let paragraph_block = Paragraph::new(self.message.clone())
            .centered()
            .block(message_block);
        paragraph_block.render(rows[0], buf);

        let mut typing_showed = false;

        for i in 2..8 {
            let centered = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(total_cells_width),
                    Constraint::Min(0),
                ])
                .split(rows[i]);

            let cells = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(cell_width); 5])
                .split(centered[1]);

            let guess: Option<[char; 5]> = if self.guesses.len() > i - 2 {
                Some(self.guesses[i - 2])
            } else {
                None
            };

            if let Some(word) = guess {
                let mut done: HashMap<char, usize> = HashMap::new();

                for j in 0..5 {
                    if word[j] == self.correct[j] {
                        if let Some(count) = done.get_mut(&word[j]) {
                            *count = count.saturating_add(1);
                        } else {
                            done.insert(word[j], 1usize);
                        }
                    }
                }

                for j in 0..5 {
                    let cell_block;

                    if word[j] == self.correct[j] {
                        cell_block = Block::bordered().green();
                    } else {
                        let mut count = self.correct.iter().filter(|x| x == &&word[j]).count();

                        if let Some(has) = done.get(&word[j]) {
                            count = count.saturating_sub(*has);
                        }

                        cell_block = if count > 0 {
                            done.entry(word[j]).and_modify(|e| *e += 1).or_insert(1);
                            Block::bordered().yellow()
                        } else {
                            Block::bordered().red()
                        };
                    }

                    let paragraph = Paragraph::new(word[j].to_string())
                        .centered()
                        .block(cell_block);
                    paragraph.render(cells[j], buf);
                }
            } else {
                for j in 0..5 {
                    if !typing_showed && let Some(c) = self.typing[j] {
                        let cell_block = Block::bordered().blue();
                        let paragraph = Paragraph::new(c.to_string()).centered().block(cell_block);
                        paragraph.render(cells[j], buf);
                    } else {
                        if !typing_showed {
                            Block::bordered().light_blue().render(cells[j], buf);
                        } else {
                            Block::bordered().render(cells[j], buf);
                        }
                    }
                }

                if !typing_showed {
                    typing_showed = true
                }
            }
        }
    }
}
