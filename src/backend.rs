use ratatui::backend::Backend;
use ratatui::layout::{Rect, Size};
use ratatui::style::{Color, Modifier};
use std::io;
use unicode_width::UnicodeWidthStr;

type WriteBytesFn = dyn FnMut(&[u8]);

/// Ratatui backend implementation for SSH terminal output.
///
/// This backend converts Ratatui drawing commands into ANSI escape sequences
/// that are sent to SSH clients. It implements the full Backend trait to support
/// colors, modifiers, cursor control, and terminal dimensions.
pub struct SSHUIBackend {
    /// A write function that sends bytes to the SSH client.
    pub write: Box<WriteBytesFn>,
    /// The terminal dimensions (width and height in characters).
    pub size: Rect,
}

fn color_to_ansi_fg(color: Color) -> String {
    match color {
        Color::Reset => "\x1b[39m".to_string(),
        Color::Black => "\x1b[30m".to_string(),
        Color::Red => "\x1b[31m".to_string(),
        Color::Green => "\x1b[32m".to_string(),
        Color::Yellow => "\x1b[33m".to_string(),
        Color::Blue => "\x1b[34m".to_string(),
        Color::Magenta => "\x1b[35m".to_string(),
        Color::Cyan => "\x1b[36m".to_string(),
        Color::Gray => "\x1b[37m".to_string(),
        Color::DarkGray => "\x1b[90m".to_string(),
        Color::LightRed => "\x1b[91m".to_string(),
        Color::LightGreen => "\x1b[92m".to_string(),
        Color::LightYellow => "\x1b[93m".to_string(),
        Color::LightBlue => "\x1b[94m".to_string(),
        Color::LightMagenta => "\x1b[95m".to_string(),
        Color::LightCyan => "\x1b[96m".to_string(),
        Color::White => "\x1b[97m".to_string(),
        Color::Rgb(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
        Color::Indexed(i) => format!("\x1b[38;5;{i}m"),
    }
}

fn color_to_ansi_bg(color: Color) -> String {
    match color {
        Color::Reset => "\x1b[49m".to_string(),
        Color::Black => "\x1b[40m".to_string(),
        Color::Red => "\x1b[41m".to_string(),
        Color::Green => "\x1b[42m".to_string(),
        Color::Yellow => "\x1b[43m".to_string(),
        Color::Blue => "\x1b[44m".to_string(),
        Color::Magenta => "\x1b[45m".to_string(),
        Color::Cyan => "\x1b[46m".to_string(),
        Color::Gray => "\x1b[47m".to_string(),
        Color::DarkGray => "\x1b[100m".to_string(),
        Color::LightRed => "\x1b[101m".to_string(),
        Color::LightGreen => "\x1b[102m".to_string(),
        Color::LightYellow => "\x1b[103m".to_string(),
        Color::LightBlue => "\x1b[104m".to_string(),
        Color::LightMagenta => "\x1b[105m".to_string(),
        Color::LightCyan => "\x1b[106m".to_string(),
        Color::White => "\x1b[107m".to_string(),
        Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
        Color::Indexed(i) => format!("\x1b[48;5;{i}m"),
    }
}

fn modifier_to_ansi(modifier: Modifier) -> String {
    let mut s = String::new();
    if modifier.contains(Modifier::BOLD) {
        s.push_str("\x1b[1m");
    }
    if modifier.contains(Modifier::DIM) {
        s.push_str("\x1b[2m");
    }
    if modifier.contains(Modifier::ITALIC) {
        s.push_str("\x1b[3m");
    }
    if modifier.contains(Modifier::UNDERLINED) {
        s.push_str("\x1b[4m");
    }
    if modifier.contains(Modifier::SLOW_BLINK) || modifier.contains(Modifier::RAPID_BLINK) {
        s.push_str("\x1b[5m");
    }
    if modifier.contains(Modifier::REVERSED) {
        s.push_str("\x1b[7m");
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        s.push_str("\x1b[9m");
    }
    s
}

impl Backend for SSHUIBackend {
    type Error = io::Error;

    fn draw<'b, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'b ratatui::buffer::Cell)>,
    {
        let mut cells: Vec<(u16, u16, &ratatui::buffer::Cell)> = content.collect();
        cells.sort_by_key(|(x, y, _)| (*y, *x));

        let mut current_y = None;
        let mut current_x = 0u16;
        for (x, y, cell) in cells {
            if current_y != Some(y) {
                let cursor_cmd = format!("\x1b[{};{}H", y + 1, 1);
                (self.write)(cursor_cmd.as_bytes());
                current_y = Some(y);
                current_x = 0;
            }

            while current_x < x {
                (self.write)(b" ");
                current_x += 1;
            }

            (self.write)(b"\x1b[0m");
            (self.write)(color_to_ansi_fg(cell.fg).as_bytes());
            (self.write)(color_to_ansi_bg(cell.bg).as_bytes());
            (self.write)(modifier_to_ansi(cell.modifier).as_bytes());

            let symbol = cell.symbol();
            (self.write)(symbol.as_bytes());
            (self.write)(b"\x1b[0m");

            let width = UnicodeWidthStr::width(symbol);
            current_x = x + width as u16;
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        (self.write)(b"\x1b[?25l");
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        (self.write)(b"\x1b[?25h");
        Ok(())
    }

    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        Ok((0, 0))
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        let cmd = format!("\x1b[{};{}H", y + 1, x + 1);
        (self.write)(cmd.as_bytes());
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        (self.write)(b"\x1b[2J\x1b[H");
        Ok(())
    }

    fn size(&self) -> Result<Size, Self::Error> {
        let rect = self.size;
        Ok(Size {
            width: rect.width,
            height: rect.height,
        })
    }

    fn get_cursor_position(&mut self) -> Result<ratatui::prelude::Position, Self::Error> {
        // cannot get the position through SSH
        Ok(ratatui::prelude::Position { x: 0, y: 0 })
    }

    fn set_cursor_position<P: Into<ratatui::prelude::Position>>(
        &mut self,
        position: P,
    ) -> Result<(), Self::Error> {
        let pos = position.into();
        let cmd = format!("\x1b[{};{}H", pos.y + 1, pos.x + 1);
        (self.write)(cmd.as_bytes());
        Ok(())
    }

    fn clear_region(
        &mut self,
        clear_type: ratatui::prelude::backend::ClearType,
    ) -> Result<(), Self::Error> {
        use ratatui::prelude::backend::ClearType;
        let seq = match clear_type {
            ClearType::All => b"\x1b[2J".as_slice(),
            ClearType::AfterCursor => b"\x1b[J",
            ClearType::BeforeCursor => b"\x1b[1J",
            ClearType::CurrentLine => b"\x1b[2K",
            ClearType::UntilNewLine => b"\x1b[K",
        };
        (self.write)(seq);
        Ok(())
    }

    fn window_size(&mut self) -> Result<ratatui::prelude::backend::WindowSize, Self::Error> {
        Ok(ratatui::prelude::backend::WindowSize {
            columns_rows: self.size()?,
            pixels: ratatui::layout::Size {
                width: 0,
                height: 0,
            },
        })
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn append_lines(&mut self, _n: u16) -> Result<(), Self::Error> {
        Ok(())
    }
}
