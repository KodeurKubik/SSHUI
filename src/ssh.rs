use anyhow::Result;
use ratatui::Terminal;
use russh::server::{Auth, Handle, Handler, Msg, Session};
use russh::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use termwiz::input::InputParser;

use crate::auth::{AuthMethod, auth_to_decision};
use crate::backend::SSHUIBackend;
use crate::{App, AuthHandler};

/// The SSH server that hosts SSHUI applications.
///
/// This struct implements the `russh::server::Server` trait and manages SSH connections,
/// creating a new application instance for each connected client.
pub struct SSHUIServer {
    /// A factory function that creates a new App instance for each client connection.
    pub app_factory: Arc<dyn Fn() -> Box<dyn App> + Send + Sync>,
    /// Atomic counter tracking the number of currently connected clients.
    pub connected_clients: Arc<AtomicUsize>,
    /// A method to handle auth
    pub auth: Arc<dyn AuthHandler>,
    /// Optional refresh rate for periodic re-rendering
    pub refresh_rate: Option<Duration>,
}

impl server::Server for SSHUIServer {
    type Handler = SSHUIHandler;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        let app = (self.app_factory)();

        SSHUIHandler {
            channel: None,
            cols: Arc::new(AtomicU32::new(0)),
            rows: Arc::new(AtomicU32::new(0)),
            term: None,
            app: Arc::new(Mutex::new(Some(app))),
            input_parser: InputParser::new(),
            connected_clients: self.connected_clients.clone(),
            auth: self.auth.clone(),
            authenticated: false,
            refresh_rate: self.refresh_rate,
            refresh_running: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// SSH session handler that manages individual client connections.
///
/// This struct handles the SSH protocol implementation for a single connected client,
/// managing terminal requests, input/output, and application lifecycle.
pub struct SSHUIHandler {
    /// The SSH channel ID for this session.
    channel: Option<ChannelId>,
    /// Terminal width in columns.
    cols: Arc<AtomicU32>,
    /// Terminal height in rows.
    rows: Arc<AtomicU32>,
    /// Terminal type
    term: Option<String>,
    /// The application instance for this client (shared for refresh task).
    app: Arc<Mutex<Option<Box<dyn App>>>>,
    /// Parser for terminal input sequences.
    input_parser: InputParser,
    /// Shared counter of connected clients.
    connected_clients: Arc<AtomicUsize>,
    /// A method to handle auth
    auth: Arc<dyn AuthHandler>,
    /// Whether this connection has been authenticated (and thus counted)
    authenticated: bool,
    /// Optional refresh rate for periodic re-rendering
    refresh_rate: Option<Duration>,
    /// Flag to stop refresh task on disconnect
    refresh_running: Arc<AtomicBool>,
}

impl SSHUIHandler {
    fn render(&mut self, channel: ChannelId, session: &mut Session) -> Result<()> {
        let cols = self.cols.load(Ordering::SeqCst);
        let rows = self.rows.load(Ordering::SeqCst);

        let output = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output.clone();

        let write = move |bytes: &[u8]| {
            if let Ok(mut buf) = output_clone.lock() {
                buf.extend_from_slice(bytes);
            }
        };

        let size = ratatui::layout::Rect::new(0, 0, cols as u16, rows as u16);

        let backend = SSHUIBackend {
            write: Box::new(write),
            size,
        };

        let mut terminal = Terminal::new(backend)?;

        let should_close = if let Ok(mut app_guard) = self.app.lock() {
            if let Some(app) = app_guard.as_mut() {
                app.render(&mut terminal)?
            } else {
                None
            }
        } else {
            None
        };

        if let Some(exit_message) = should_close {
            let _ = self.close(channel, session, Some(exit_message));
            return Ok(());
        }

        if let Ok(buf) = output.lock() {
            let _ = session.data(channel, buf.clone().into());
        }

        Ok(())
    }

    fn close(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
        exit_message: Option<String>,
    ) -> Result<()> {
        let _ = session.data(channel, "\x1b[?1049l\x1b[?25h\x1b[0m".into());
        let _ = session.data(
            channel,
            exit_message
                .unwrap_or("== Exited - Goodbye! ==".to_string())
                .into(),
        );
        let _ = session.data(channel, "\n\n\r".into());

        let _ = session.exit_status_request(channel, 0);
        let _ = session.eof(channel);
        let _ = session.close(channel);

        Ok(())
    }

    fn log_connected(&self) {
        let count = self.connected_clients.load(Ordering::SeqCst);

        if count == 0 {
            print!("\r\x1b[KWaiting for clients... ");
        } else {
            print!("\r\x1b[KConnected clients: {count} ");
        }
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

/// Renders the app and sends output via Handle (for background refresh task)
async fn render_to_handle(
    handle: &Handle,
    channel: ChannelId,
    app: &Arc<Mutex<Option<Box<dyn App>>>>,
    cols: u32,
    rows: u32,
) -> Result<()> {
    let output = {
        let output_buf = Arc::new(Mutex::new(Vec::new()));
        let output_clone = output_buf.clone();

        let write = move |bytes: &[u8]| {
            if let Ok(mut buf) = output_clone.lock() {
                buf.extend_from_slice(bytes);
            }
        };

        let size = ratatui::layout::Rect::new(0, 0, cols as u16, rows as u16);
        let backend = SSHUIBackend {
            write: Box::new(write),
            size,
        };
        let mut terminal = Terminal::new(backend)?;

        if let Ok(mut app_guard) = app.lock() {
            if let Some(app) = app_guard.as_mut() {
                app.on_tick();
                let _ = app.render(&mut terminal);
            }
        }

        output_buf
            .lock()
            .ok()
            .map(|b| b.clone())
            .unwrap_or_default()
    };

    handle
        .data(channel, output.into())
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;

    Ok(())
}

impl Handler for SSHUIHandler {
    type Error = anyhow::Error;

    /// Handles PTY (pseudo-terminal) allocation request.
    ///
    /// Called when the client requests a PTY with specific dimensions and terminal type.
    /// Stores the terminal parameters for use during rendering.
    async fn pty_request(
        &mut self,
        channel: ChannelId,
        term: &str,
        cols: u32,
        rows: u32,
        _px_width: u32,
        _px_height: u32,
        _modes: &[(Pty, u32)],
        _session: &mut Session,
    ) -> Result<()> {
        self.channel = Some(channel);
        self.cols.store(cols, Ordering::SeqCst);
        self.rows.store(rows, Ordering::SeqCst);
        self.term = Some(term.to_string());

        Ok(())
    }

    /// Increments the client counter and logs the new connection status.
    async fn auth_succeeded(&mut self, _session: &mut Session) -> Result<()> {
        self.authenticated = true;
        self.connected_clients
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.log_connected();

        Ok(())
    }

    /// Handles authentication with username but no password
    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        Ok(auth_to_decision(
            self.auth.auth_none(user).await,
            AuthMethod::None,
        ))
    }

    /// Handles authentication with username and password
    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> std::result::Result<Auth, Self::Error> {
        Ok(auth_to_decision(
            self.auth.auth_password(user, password).await,
            AuthMethod::Password,
        ))
    }

    /// Handles opening a new SSH session channel.
    ///
    /// Accepts the channel open request.
    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    /// Handles shell request and initializes the application.
    ///
    /// Sets up the alternative screen buffer, hides the cursor, and renders
    /// the application for the first time.
    async fn shell_request(&mut self, channel: ChannelId, session: &mut Session) -> Result<()> {
        self.channel = Some(channel);

        let _ = session.channel_success(channel);
        let _ = session.data(channel, "\x1b[?1049h\x1b[H\x1b[?25l".into());

        self.render(channel, session)?;

        // Start periodic refresh task if refresh_rate is set
        if let Some(refresh_rate) = self.refresh_rate {
            let handle = session.handle();
            let running = self.refresh_running.clone();
            let app = self.app.clone();
            let cols = self.cols.clone();
            let rows = self.rows.clone();
            running.store(true, Ordering::SeqCst);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(refresh_rate);
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                // Skip the first immediate tick
                interval.tick().await;

                while running.load(Ordering::SeqCst) {
                    interval.tick().await;
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }

                    // Render directly from background task
                    if let Err(_) = render_to_handle(
                        &handle,
                        channel,
                        &app,
                        cols.load(Ordering::SeqCst),
                        rows.load(Ordering::SeqCst),
                    )
                    .await
                    {
                        break;
                    }
                }
            });
        }

        Ok(())
    }

    /// Handles data received from the SSH client.
    ///
    /// Parses terminal input sequences, handles Ctrl+C to exit, passes other
    /// input events to the application, and re-renders the UI.
    async fn data(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) -> Result<()> {
        let mut events = Vec::new();
        self.input_parser.parse(data, |e| events.push(e), false);

        for event in events {
            if let termwiz::input::InputEvent::Key(termwiz::input::KeyEvent {
                key: termwiz::input::KeyCode::Char('c'),
                modifiers: termwiz::input::Modifiers::CTRL,
            }) = &event
            {
                let _ = self.close(channel, session, None);
                return Ok(());
            }

            if let Ok(mut app_guard) = self.app.lock() {
                if let Some(app) = app_guard.as_mut() {
                    app.input(event);
                }
            }
        }

        self.render(channel, session)?;
        Ok(())
    }

    /// Handles terminal window resize requests.
    ///
    /// Updates the stored terminal dimensions, clears the screen, and re-renders
    /// the application with the new size.
    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        cols: u32,
        rows: u32,
        _px_width: u32,
        _px_height: u32,
        session: &mut Session,
    ) -> Result<()> {
        self.cols.store(cols, Ordering::SeqCst);
        self.rows.store(rows, Ordering::SeqCst);
        let _ = session.data(channel, "\x1b[2J\x1b[H".into());
        self.render(channel, session)?;
        Ok(())
    }
}

impl Drop for SSHUIHandler {
    fn drop(&mut self) {
        // Stop the refresh task
        self.refresh_running.store(false, Ordering::SeqCst);

        // Only decrement if we previously incremented (after successful auth)
        if self.authenticated {
            self.connected_clients.fetch_sub(1, Ordering::SeqCst);
            self.log_connected();
        }
    }
}
