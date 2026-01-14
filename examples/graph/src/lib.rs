use anyhow::Result;
use sshui::ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Widget,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Axis, Block, Chart, Dataset, Paragraph},
};
use sshui::{InputEvent, KeyCode, KeyEvent, SSHUITerminal};

#[derive(Debug)]
pub struct GraphApp {
    pub viewport: Viewport,
    pub input_f: String,
    pub input_g: String,
    pub selected: u8,
    pub char: usize,
    pub exit: bool,
}

impl GraphApp {
    pub fn default() -> Self {
        Self {
            viewport: Viewport::default(),
            input_f: "x".to_string(),
            input_g: String::new(),
            char: 1,
            selected: 1,
            exit: false,
        }
    }

    fn generate_f_data(&self) -> Result<Vec<(f64, f64)>> {
        if self.input_f.trim() == "" {
            return Err(anyhow::anyhow!("Nothing to parse"));
        }

        let expr: meval::Expr = self.input_f.parse()?;
        let func = expr.bind("x")?;
        let samples = self.viewport.optimal_sample_count();

        Ok(self
            .viewport
            .x_range(samples)
            .map(|x| (x, func(x)))
            .filter(|x| x.1.is_finite())
            .collect())
    }

    fn generate_g_data(&self) -> Result<Vec<(f64, f64)>> {
        if self.input_g.trim() == "" {
            return Err(anyhow::anyhow!("Nothing to parse"));
        }

        let expr: meval::Expr = self.input_g.parse()?;
        let func = expr.bind("x")?;
        let samples = self.viewport.optimal_sample_count();

        Ok(self
            .viewport
            .x_range(samples)
            .map(|x| (x, func(x)))
            .filter(|x| x.1.is_finite())
            .collect())
    }
}

impl sshui::App for GraphApp {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
        terminal.draw(|frame| {
            self.render_widget(frame.area(), frame.buffer_mut());
        })?;

        Ok(if self.exit {
            Some("Exited".to_string())
        } else {
            None
        })
    }

    fn input(&mut self, event: InputEvent) {
        let InputEvent::Key(KeyEvent { key, modifiers, .. }) = event else {
            return;
        };

        match key {
            KeyCode::Escape => self.exit = true,
            KeyCode::Tab | KeyCode::Enter => {
                if modifiers.contains(sshui::Modifiers::SHIFT) {
                    if self.selected == 0 {
                        self.selected = 2;
                    } else {
                        self.selected -= 1;
                    }
                } else {
                    if self.selected >= 2 {
                        self.selected = 0;
                    } else {
                        self.selected += 1;
                    }
                }

                self.char = if self.selected == 1u8 {
                    self.input_f.len()
                } else {
                    self.input_g.len()
                };
            }
            KeyCode::Backspace => {
                if self.selected != 0u8 {
                    let text = if self.selected == 1u8 {
                        &mut self.input_f
                    } else {
                        &mut self.input_g
                    };

                    if modifiers.contains(sshui::Modifiers::CTRL)
                        || modifiers.contains(sshui::Modifiers::SUPER)
                    {
                        text.clear();
                        self.char = 0;
                    } else if self.char > 0 && text.len() >= self.char {
                        self.char -= 1;
                        text.remove(self.char);
                    }
                }
            }
            KeyCode::Char(mut c) => {
                if self.selected == 0u8 {
                    if c == '0' || c == 'à' {
                        // reset viewport, à for azerty again
                        self.viewport = Viewport::default();
                        return;
                    }

                    let zoom_factor = if c == '-' {
                        1.5
                    } else if c == '+' || c == '=' {
                        1.0 / 1.5 // Zoom in ('=' for azerty keyboards lol)
                    } else {
                        1.0 // No zoom
                    };

                    if zoom_factor != 1.0 {
                        let x_center = (self.viewport.x_min + self.viewport.x_max) / 2.0;
                        let y_center = (self.viewport.y_min + self.viewport.y_max) / 2.0;
                        let x_range =
                            (self.viewport.x_max - self.viewport.x_min) * zoom_factor / 2.0;
                        let y_range =
                            (self.viewport.y_max - self.viewport.y_min) * zoom_factor / 2.0;

                        self.viewport.x_min = x_center - x_range;
                        self.viewport.x_max = x_center + x_range;
                        self.viewport.y_min = y_center - y_range;
                        self.viewport.y_max = y_center + y_range;
                    }
                } else {
                    if modifiers.contains(sshui::Modifiers::CTRL) && c == 'u' {
                        let text = if self.selected == 1u8 {
                            &mut self.input_f
                        } else {
                            &mut self.input_g
                        };
                        text.clear();
                        self.char = 0;
                        return;
                    }

                    if modifiers.contains(sshui::Modifiers::CTRL)
                        || modifiers.contains(sshui::Modifiers::SUPER)
                        || modifiers.contains(sshui::Modifiers::ALT)
                    {
                        return;
                    }

                    if c.is_ascii() {
                        c = c.to_ascii_lowercase();

                        if self.selected == 1u8 {
                            self.input_f.insert(self.char, c);
                        } else if self.selected == 2u8 {
                            self.input_g.insert(self.char, c);
                        }
                        self.char += 1;
                    }
                }
            }
            KeyCode::LeftArrow => {
                if self.selected == 0u8 {
                    let x_step = (self.viewport.x_max - self.viewport.x_min) * 0.1;
                    self.viewport.x_max -= x_step;
                    self.viewport.x_min -= x_step;
                } else {
                    self.char = self.char.saturating_sub(1);
                }
            }
            KeyCode::RightArrow => {
                if self.selected == 0u8 {
                    let x_step = (self.viewport.x_max - self.viewport.x_min) * 0.1;
                    self.viewport.x_max += x_step;
                    self.viewport.x_min += x_step;
                } else {
                    let text = if self.selected == 1u8 {
                        &self.input_f
                    } else {
                        &self.input_g
                    };

                    if self.char < text.len() {
                        self.char += 1;
                    }
                }
            }
            KeyCode::UpArrow => {
                if self.selected == 0u8 {
                    let y_step = (self.viewport.y_max - self.viewport.y_min) * 0.1;
                    self.viewport.y_max += y_step;
                    self.viewport.y_min += y_step;
                }
            }
            KeyCode::DownArrow => {
                if self.selected == 0u8 {
                    let y_step = (self.viewport.y_max - self.viewport.y_min) * 0.1;
                    self.viewport.y_max -= y_step;
                    self.viewport.y_min -= y_step;
                }
            }
            _ => {}
        }
    }
}

impl GraphApp {
    fn render_widget(&self, area: Rect, buf: &mut Buffer) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_chart(sections[0], buf);
        self.render_input_section(sections[1], buf);
        self.render_controls(sections[2], buf);
    }

    fn render_chart(&self, area: Rect, buf: &mut Buffer) {
        let vp = &self.viewport;
        let x_mid = (vp.x_min + vp.x_max) / 2.0;
        let y_mid = (vp.y_min + vp.y_max) / 2.0;

        let x_labels = vec![
            Span::styled(
                format!("{:.0}", vp.x_min),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.0}", x_mid)),
            Span::styled(
                format!("{:.0}", vp.x_max),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];

        let y_labels = vec![
            Span::styled(
                format!("{:.0}", vp.y_min),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.0}", y_mid)),
            Span::styled(
                format!("{:.0}", vp.y_max),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];

        let data1 = self.generate_f_data().ok();
        let data2 = self.generate_g_data().ok();

        let samples = self.viewport.optimal_sample_count();
        let y_axis_data: Vec<(f64, f64)> = (0..=samples)
            .map(|i| {
                let t = i as f64 / samples as f64;
                (0.0, vp.y_min + (vp.y_max - vp.y_min) * t)
            })
            .collect();

        let x_axis_data: Vec<(f64, f64)> = (0..=samples)
            .map(|i| {
                let t = i as f64 / samples as f64;
                (vp.x_min + (vp.x_max - vp.x_min) * t, 0.0)
            })
            .collect();

        let mut datasets = Vec::with_capacity(4);

        if vp.x_min <= 0.0 && vp.x_max >= 0.0 {
            datasets.push(
                Dataset::default()
                    .marker(sshui::ratatui::symbols::Marker::Braille)
                    .style(Style::default().fg(Color::DarkGray))
                    .data(&y_axis_data),
            );
        }
        if vp.y_min <= 0.0 && vp.y_max >= 0.0 {
            datasets.push(
                Dataset::default()
                    .marker(sshui::ratatui::symbols::Marker::Braille)
                    .style(Style::default().fg(Color::DarkGray))
                    .data(&x_axis_data),
            );
        }

        if let Some(ref data1) = data1 {
            datasets.push(
                Dataset::default()
                    .name("f(x)")
                    .marker(sshui::ratatui::symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Cyan))
                    .data(data1),
            )
        }
        if let Some(ref data2) = data2 {
            datasets.push(
                Dataset::default()
                    .name("g(x)")
                    .marker(sshui::ratatui::symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Yellow))
                    .data(data2),
            )
        }

        let chart = Chart::new(datasets)
            .block(Block::bordered())
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::White))
                    .labels(x_labels)
                    .bounds([vp.x_min, vp.x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::White))
                    .labels(y_labels)
                    .bounds([vp.y_min, vp.y_max]),
            );

        chart.render(area, buf);
    }

    fn render_input_section(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered().title("Functions");
        let inner = block.inner(area);
        block.render(area, buf);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        self.render_input_line(1u8, "f(x)", &self.input_f, Color::Cyan, rows[0], buf);
        self.render_input_line(2u8, "g(x)", &self.input_g, Color::Yellow, rows[1], buf);
    }

    fn render_input_line(
        &self,
        id: u8,
        label: &str,
        input: &str,
        color: Color,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(7),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(area);

        let label_text = Paragraph::new(format!("{} =", label)).style(Style::default().fg(color));
        label_text.render(parts[0], buf);

        if let Some(cell) = buf.cell_mut((parts[1].left(), parts[1].top())) {
            cell.set_char('│');

            cell.set_fg(if id == self.selected {
                Color::Green
            } else {
                Color::DarkGray
            });
        }

        if id == self.selected {
            let (part1, part2) = input.split_at(self.char);
            let cursor_char = part2.chars().next().unwrap_or(' ');
            let rest = if part2.is_empty() {
                String::new()
            } else {
                part2[cursor_char.len_utf8()..].to_string()
            };

            let input_text = Paragraph::new(format!(" {}{}{}", part1, cursor_char, rest))
                .style(Style::default().fg(Color::White));

            input_text.render(parts[2], buf);

            if let Some(cell) =
                buf.cell_mut((parts[2].left() + 1 + part1.len() as u16, parts[2].top()))
            {
                cell.set_fg(Color::Green);
                cell.set_style(Style::new().add_modifier(Modifier::UNDERLINED));
            }
        } else {
            let input_text =
                Paragraph::new(format!(" {input}")).style(Style::default().fg(Color::White));
            input_text.render(parts[2], buf);
        }
    }

    fn render_controls(&self, area: Rect, buf: &mut Buffer) {
        let controls = vec![
            Span::raw("  Unselect and ["),
            Span::styled("←→↑↓", Style::default().fg(Color::Yellow).bold()),
            Span::raw("] Move  ["),
            Span::styled("+-", Style::default().fg(Color::Yellow).bold()),
            Span::raw("] Zoom  ["),
            Span::styled("0", Style::default().fg(Color::Yellow).bold()),
            Span::raw("] Reset Zoom  ["),
            Span::styled("Tab/↵", Style::default().fg(Color::Yellow).bold()),
            Span::raw("] Switch  ["),
            Span::styled("ESC", Style::default().fg(Color::Yellow).bold()),
            Span::raw("] Exit"),
        ];
        let controls_line = Line::from(controls);
        let paragraph = Paragraph::new(controls_line);
        paragraph.render(area, buf);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x_min: -100.0,
            x_max: 100.0,
            y_min: -100.0,
            y_max: 100.0,
        }
    }
}

impl Viewport {
    pub fn x_range(&self, steps: usize) -> impl Iterator<Item = f64> {
        let step = (self.x_max - self.x_min) / steps as f64;
        let x_min = self.x_min;
        (0..=steps).map(move |i| x_min + (i as f64) * step)
    }

    pub fn optimal_sample_count(&self) -> usize {
        let range = (self.x_max - self.x_min).abs();
        let samples = (range * 10.0).max(500.0).min(5000.0) as usize;
        samples
    }
}
