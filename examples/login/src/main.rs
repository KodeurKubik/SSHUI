use anyhow::Result;
use sshui::{
    self, AuthDecision, AuthHandler, InputEvent, KeyCode, KeyEvent, SSHUITerminal,
    ratatui::{
        buffer::Buffer,
        layout::Rect,
        widgets::{Paragraph, Widget},
    },
};

struct MyAuth;

#[async_trait::async_trait]
impl AuthHandler for MyAuth {
    async fn auth_password(&self, user: &str, password: &str) -> AuthDecision {
        // absolutely secured password (trust)
        if user == "hello" && password == "world" {
            AuthDecision::Accept
        } else {
            AuthDecision::Reject
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    let key_pair = sshui::get_debug_ssh_key().unwrap();
    #[cfg(not(debug_assertions))]
    let key_pair = sshui::get_ssh_key().unwrap();

    let config = sshui::Config {
        keys: vec![key_pair],
        ..Default::default()
    };

    sshui::new_server_with_password(
        config,
        ("0.0.0.0", 2222),
        || Box::new(App::default()),
        MyAuth,
    )
    .await
    .unwrap();

    Ok(())
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
}

impl sshui::App for App {
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
        terminal.draw(|frame| {
            frame.render_widget(&*self, frame.area());
        })?;

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
            KeyCode::Char('q') | KeyCode::Escape => self.exit = true,
            _ => {}
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Welcome In!\nPress Q or ESC to quit").render(area, buf);
    }
}
