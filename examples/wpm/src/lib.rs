use anyhow::Result;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use sshui::ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::Stylize,
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
use sshui::{InputEvent, KeyCode, KeyEvent, SSHUITerminal};
use std::time::{Duration, SystemTime};

#[derive(Deserialize, Serialize)]
struct Quote {
    text: String,
    source: String,
    length: u32,
    id: u32,
}

#[derive(Deserialize, Serialize)]
struct EnglishData {
    language: String,
    groups: Vec<[u32; 2]>,
    quotes: Vec<Quote>,
}

const MAX_LENGTH_PER_LINE: usize = 50;
const ENGLISH_JSON: &str = include_str!("english.json");

pub struct WPMApp {
    start: SystemTime,

    correct: u32,
    incorrect: u32,
    words: u32,

    current_line: usize,
    selected_group: usize,
    groups: Vec<[u32; 2]>,

    sentence: Vec<String>,
    sentence_source: String,
    typed: Vec<String>,
    typing: Vec<char>,

    exit: bool,
    done: Option<SystemTime>,
}

impl sshui::App for WPMApp {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
        terminal.draw(|frame| {
            self.render_widget(frame.area(), frame.buffer_mut());
        })?;

        if self.exit {
            Ok(Some("Goodbye! Thanks for typing :p".to_string()))
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
                self.exit = true;
            }
            KeyCode::LeftArrow => {
                if self.typing.len() != 0 || self.current_line != 0 {
                    return;
                }

                if self.selected_group == 0 {
                    self.selected_group = self.groups.len().saturating_sub(1);
                } else {
                    self.selected_group -= 1;
                }
                self.new_quote();
            }
            KeyCode::RightArrow => {
                if self.typing.len() != 0 || self.current_line != 0 {
                    return;
                }

                if self.selected_group + 1 >= self.groups.len() {
                    self.selected_group = 0;
                } else {
                    self.selected_group += 1;
                }
                self.new_quote();
            }
            KeyCode::Tab => {
                if self.typing.len() == 0 && self.current_line == 0 {
                    return self.new_quote();
                }

                if self.done.is_some() {
                    self.correct = 0;
                    self.incorrect = 0;
                    self.words = 0;

                    self.current_line = 0;
                    self.typing = Vec::with_capacity(MAX_LENGTH_PER_LINE);
                    self.typed = Vec::new();

                    self.done = None;

                    self.new_quote();
                }
            }
            KeyCode::Char(char) => {
                if self.typing.len() == 0 && self.current_line == 0 {
                    self.start = SystemTime::now()
                }

                let part = &self.sentence[self.current_line];

                if char.is_whitespace() {
                    if part.chars().nth(self.typing.len()) != Some(' ') {
                        return;
                    }

                    self.words += 1;
                }
                if !char.is_whitespace() && part.chars().nth(self.typing.len()) == Some(' ') {
                    return;
                }

                self.typing.push(char);

                if part.len() == self.typing.len() {
                    self.count_mistakes();

                    self.typed.push(self.typing.iter().collect::<String>());

                    self.typing = Vec::with_capacity(MAX_LENGTH_PER_LINE);
                    self.current_line += 1;

                    if self.current_line + 1 > self.sentence.len() {
                        return self.done = Some(SystemTime::now());
                    }
                }
            }
            KeyCode::Backspace => {
                let char = self.typing.pop();
                if let Some(is) = char {
                    if is.is_whitespace() {
                        self.words = self.words.saturating_sub(1);
                    }
                }
            }
            _ => {}
        }
    }
}

impl WPMApp {
    pub fn default() -> Self {
        let mut app = WPMApp {
            start: SystemTime::now(),

            correct: 0,
            incorrect: 0,
            words: 0,

            selected_group: 0,
            current_line: 0,
            groups: Vec::with_capacity(4),

            sentence: Vec::new(),
            sentence_source: "loading quote...".to_string(),
            typing: Vec::with_capacity(MAX_LENGTH_PER_LINE),
            typed: Vec::new(),

            exit: false,
            done: None,
        };

        app.new_quote();

        app
    }

    fn new_quote(&mut self) {
        let data: EnglishData =
            serde_json::from_str(ENGLISH_JSON).expect("Failed to parse english.json");
        let mut rng = rand::rng();

        self.groups = data.groups.clone();

        if self.selected_group > data.groups.len() {
            self.selected_group = 0;
        }

        let group = data.groups[self.selected_group];
        let valid_quotes: Vec<&Quote> = data
            .quotes
            .iter()
            .filter(|q| group[0] < q.length && q.length < group[1])
            .collect();

        let picked = valid_quotes
            .choose(&mut rng)
            .expect("Could not pick a quote");

        self.sentence_source = picked.source.clone();
        self.sentence = Vec::new();

        for word in picked.text.split(" ") {
            if let Some(l) = self.sentence.last_mut() {
                if l.len() + 1 + word.len() > MAX_LENGTH_PER_LINE {
                    self.sentence.push(word.to_string());
                } else {
                    l.push_str(" ");
                    l.push_str(word);
                }
            } else {
                self.sentence.push(word.to_string());
            }
        }

        for i in 0..self.sentence.len().saturating_sub(1) {
            self.sentence[i].push_str(" ");
        }
    }

    fn count_mistakes(&mut self) {
        for i in 0..self.typing.len() {
            if self.typing[i]
                == self.sentence[self.current_line]
                    .chars()
                    .nth(i)
                    .unwrap_or(' ')
            {
                self.correct += 1;
            } else {
                self.incorrect += 1;
            }
        }
    }

    fn render_widget(&self, area: Rect, buf: &mut Buffer) {
        if let Some(end) = self.done {
            let title = Line::from(" Typing Test Completed ".bold().green());

            let duration = end
                .duration_since(self.start)
                .unwrap_or(Duration::from_secs(0));
            let seconds = duration.as_secs_f32();
            let accuracy = if self.correct + self.incorrect > 0 {
                (self.correct as f32 / (self.correct + self.incorrect) as f32) * 100.
            } else {
                0.
            };

            let block = Block::bordered()
                .title(title.centered())
                .title_bottom(
                    Line::from(vec![
                        " Press ".into(),
                        "<ESC>".blue().bold(),
                        " to exit or ".into(),
                        "<TAB>".blue().bold(),
                        " to try again".into(),
                    ])
                    .centered(),
                )
                .border_set(border::ROUNDED);

            let inner = block.inner(area);
            block.render(area, buf);

            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(2), Constraint::Min(1)])
                .split(inner);

            let stats = vec![
                Line::from(vec![
                    "WPM: ".blue().bold(),
                    get_wpm(self.words, self.start, end).green().bold(),
                ])
                .centered(),
                Line::from(""),
                Line::from(vec![
                    "Time: ".blue().bold(),
                    format!("{:.1}s", seconds).white(),
                ])
                .centered(),
                Line::from(vec![
                    "Words: ".blue().bold(),
                    format!("{}", self.words).white(),
                ])
                .centered(),
                Line::from(""),
                Line::from(vec![
                    "Accuracy: ".blue().bold(),
                    format!("{:.1}%", accuracy).white(),
                ])
                .centered(),
                Line::from(vec![
                    "Correct: ".green().bold(),
                    format!("{}", self.correct).white(),
                    "  |  ".into(),
                    "Incorrect: ".red().bold(),
                    format!("{}", self.incorrect).white(),
                ])
                .centered(),
            ];

            Paragraph::new(stats).render(rows[1], buf);

            return;
        }

        let title = Line::from(" Typing Test ".bold());
        let instructions = Line::from(vec![
            " Start typing to ".into(),
            "<start>".blue().bold(),
            " Change quote length ".into(),
            "← →".blue().bold(),
            " New quote ".into(),
            "<TAB>".blue().bold(),
            " Quit ".into(),
            "<ESC> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // length selection + blank
                Constraint::Min(1),    // quote
                Constraint::Length(4), // blank + WPM + blank + source
            ])
            .split(inner);

        // ROW 1: Length selection || previous text if typing
        if self.typing.len() == 0 && self.current_line == 0 {
            let mut length_spans: Vec<Span> = Vec::with_capacity(1 + self.groups.len());
            length_spans.push("  Length: ".blue().bold());

            for gid in 0..self.groups.len() {
                length_spans.push(if self.selected_group == gid {
                    format!(" {}-{} ", self.groups[gid][0], self.groups[gid][1])
                        .underlined()
                        .bold()
                        .green()
                } else {
                    format!(" {}-{} ", self.groups[gid][0], self.groups[gid][1]).into()
                })
            }

            let length_text = Line::from(length_spans);
            Paragraph::new(length_text).render(rows[0], buf);
        } else {
            let mut lines: Vec<Line> = Vec::with_capacity(2);

            if self.current_line >= 2 {
                let mut spans: Vec<Span> = Vec::new();
                for (cid, c) in self.sentence[self.current_line - 2].chars().enumerate() {
                    if cid < self.typed[self.current_line - 2].len() {
                        let typed_char = self.typed[self.current_line - 2]
                            .chars()
                            .nth(cid)
                            .unwrap_or(' ');
                        if c == typed_char {
                            spans.push(c.to_string().gray().into());
                        } else {
                            spans.push(c.to_string().red().bold().into());
                        }
                    } else {
                        spans.push(c.to_string().gray().into());
                    }
                }
                lines.push(Line::from(spans).centered())
            }
            if self.current_line >= 1 {
                let mut spans: Vec<Span> = Vec::new();
                for (cid, c) in self.sentence[self.current_line - 1].chars().enumerate() {
                    if cid < self.typed[self.current_line - 1].len() {
                        let typed_char = self.typed[self.current_line - 1]
                            .chars()
                            .nth(cid)
                            .unwrap_or(' ');
                        if c == typed_char {
                            spans.push(c.to_string().gray().into());
                        } else {
                            spans.push(c.to_string().red().bold().into());
                        }
                    } else {
                        spans.push(c.to_string().gray().into());
                    }
                }
                lines.push(Line::from(spans).centered())
            }

            if lines.len() == 1 {
                lines.insert(0, Line::from(""))
            }

            Paragraph::new(Text::from(lines).centered()).render(rows[0], buf);
        }

        // Row 2: Quote text (centered)
        let mut quote_spans: Vec<Span> = Vec::with_capacity(self.typing.len() + 1);
        let mut correct = 0u32;
        let mut incorrect = 0u32;

        for cid in 0..self.typing.len() {
            if self.typing[cid]
                == self.sentence[self.current_line]
                    .chars()
                    .nth(cid)
                    .unwrap_or(' ')
            {
                quote_spans.push(self.typing[cid].to_string().into());
                correct += 1;
            } else {
                quote_spans.push(
                    self.sentence[self.current_line]
                        .chars()
                        .nth(cid)
                        .unwrap_or(' ')
                        .to_string()
                        .on_red(),
                );
                incorrect += 1;
            }
        }

        quote_spans.push(self.sentence[self.current_line][self.typing.len()..].gray());

        let active = Line::from(quote_spans);

        let mut all: Vec<Line> =
            Vec::with_capacity(self.sentence.len().saturating_sub(self.current_line) + 1);
        all.push(active);

        for k in (self.current_line + 1)..self.sentence.len() {
            all.push(Line::from(Span::from(self.sentence[k].clone().gray())))
        }

        Paragraph::new(all).centered().render(rows[1], buf);

        // Row 3: blank + WPM and stats + blank + source
        let wpm_text = Line::from(vec![
            "WPM: ".blue().bold(),
            get_wpm(self.words, self.start, SystemTime::now()).into(),
            "  |  ".into(),
            "Accuracy: ".blue().bold(),
            (self.correct + correct).to_string().green().bold(),
            " - ".into(),
            (self.incorrect + incorrect).to_string().red().bold(),
        ])
        .centered()
        .bold();
        Paragraph::new(vec![
            Line::from(""),
            wpm_text,
            Line::from(""),
            Line::from(vec![
                "  Source: ".blue().bold(),
                self.sentence_source.clone().italic(),
                " - Quotes provided by monkeytype.com".into(),
            ]),
        ])
        .render(rows[2], buf);
    }
}

fn get_wpm(words: u32, start: SystemTime, end: SystemTime) -> String {
    let duration = end.duration_since(start).unwrap_or(Duration::from_secs(0));

    let mut minutes = duration.as_secs_f32() / 60.;
    if minutes == 0. {
        minutes = 0.01;
    }

    let wpm = words as f32 / minutes;
    let emoji = if wpm < 10. {
        "🦥"
    } else if wpm < 25. {
        "🐌"
    } else if wpm < 50. {
        "🐢"
    } else if wpm < 75. {
        "🐇"
    } else if wpm < 100. {
        "🐆"
    } else if wpm < 125. {
        "🚄"
    } else {
        "⚡"
    };

    format!("{emoji} {:>3}", wpm.round() as u32)
}
