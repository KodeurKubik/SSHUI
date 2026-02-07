# SSHUI

A Rust framework for building interactive terminal user interfaces (TUIs) that run over SSH. Built on top of [Ratatui](https://github.com/ratatui-org/ratatui) and [russh](https://github.com/Eugeny/russh).

## Demo

https://github.com/user-attachments/assets/174a30fe-a076-4ae3-b2c0-3fea9acb4dd4

## Features

- **SSH Server** - Host your TUI application on an SSH server
- **Ratatui Integration** - Build beautiful terminal UIs using the Ratatui framework
- **Client Isolation** - Each SSH client gets its own application instance
- **ANSI Rendering** - Full support for colors and styles
- **Terminal Resizing** - Handles dynamic terminal size changes
- **Customizable Auth** - Ask for a specific username and password (or not!)
- **Backend Support** - Write your own backend to connect clients to each other!

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sshui = "0.2"
ratatui = "0.28"
anyhow = "1.0"
```

## Quick Start

Implement the `App` trait for your TUI:

```rust
use sshui::{App, SSHUITerminal, InputEvent, KeyCode, KeyEvent};
use anyhow::Result;

struct MyApp {
    counter: i32,
    exit: bool,
}

impl App for MyApp {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
        terminal.draw(|frame| {
            let area = frame.area();
            // Draw your UI here
        })?;

        Ok(if self.exit {
            Some("Exited".to_string())
        } else {
            None
        })
    }

    fn input(&mut self, event: InputEvent) {
        if let InputEvent::Key(KeyEvent { key, .. }) = event {
            match key {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Up => self.counter += 1,
                KeyCode::Down => self.counter -= 1,
                _ => {}
            }
        }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            counter: 0,
            exit: false,
        }
    }
}
```

Start the SSH server:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = sshui::Config::default();
    // also include ssh keys
    // which you can get using the keyring feature
    // and the sshui::get_ssh_key function

    sshui::new_server(config, ("0.0.0.0", 2222), || Box::new(MyApp::default()))
        .await?;

    Ok(())
}
```

Connect via SSH:

```bash
ssh -p 2222 localhost
```

Press `Ctrl+C` to exit.

## Examples

The repository includes example applications:

- **Example** - All the following examples at once :) (except login and counter)
- **Bad Apple** - The bad apple video, in ascii, from SSH! (i bet im not the first one)
- **Chat App** - Send messages to all connected users!
- **Demo** - The Ratatui Demo, adapted for SSHUI
- **Graph** - A really cool grapher in the terminal (holy moly)
- **Counter** - A simple incrementing counter with keyboard controls
- **Wordle** - A word guessing game
- **Login** - A 'Hello World' behind the username 'hello' and password 'world'

Run an example: (you can specify the port with --port {number})

```bash
cargo run -p example --release
```

Then connect via SSH:

```bash
ssh -p 2222 localhost
```

## Limitations

- Cursor position cannot be queried from SSH clients, so `get_cursor()` always returns (0, 0)
- Password authentication is disabled by default (implement `auth_password()` if needed)
- Pixel dimensions are not available over SSH

## License

© Kodeur_Kubik - Code available under the MIT License
