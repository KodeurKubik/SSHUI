//! # SSHUI
//!
//! A Rust framework for building interactive terminal user interfaces (TUIs) that run over SSH. Built on top of [Ratatui](https://github.com/ratatui-org/ratatui) and [russh](https://github.com/Eugeny/russh).
//!
//! ## Features
//!
//! - **SSH Server** - Host your TUI application on an SSH server
//! - **Ratatui Integration** - Build beautiful terminal UIs using the Ratatui framework
//! - **Client Isolation** - Each SSH client gets its own application instance
//! - **ANSI Rendering** - Full support for colors and styles
//! - **Terminal Resizing** - Handles dynamic terminal size changes
//! - **Customizable Auth** - Ask for a specific username and password (or not!)
//!
//! ## Installation
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! sshui = "0.2"
//! ratatui = "0.28"
//! anyhow = "1.0"
//! ```
//!
//! ## Quick Start
//!
//! Implement the `App` trait for your TUI:
//!
//! ```rust
//! use sshui::{App, SSHUITerminal, InputEvent, KeyCode, KeyEvent};
//! use anyhow::Result;
//!
//! struct MyApp {
//!     counter: i32,
//!     exit: bool,
//! }
//!
//! impl App for MyApp {
//!     fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
//!         terminal.draw(|frame| {
//!             let area = frame.area();
//!             // Draw your UI here
//!         })?;
//!
//!         Ok(if self.exit {
//!             Some("Exited".to_string())
//!         } else {
//!             None
//!         })
//!     }
//!
//!     fn input(&mut self, event: InputEvent) {
//!         if let InputEvent::Key(KeyEvent { key, .. }) = event {
//!             match key {
//!                 KeyCode::Char('q') => self.exit = true,
//!                 KeyCode::Up => self.counter += 1,
//!                 KeyCode::Down => self.counter -= 1,
//!                 _ => {}
//!             }
//!         }
//!     }
//! }
//!
//! impl Default for MyApp {
//!     fn default() -> Self {
//!         Self {
//!             counter: 0,
//!             exit: false,
//!         }
//!     }
//! }
//! ```
//!
//! Start the SSH server:
//!
//! ```rust
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = sshui::Config::default();
//!     // also include ssh keys
//!     // which you can get using the keyring feature
//!     // and the sshui::get_ssh_key function
//!
//!     sshui::new_server(config, ("0.0.0.0", 2222), || Box::new(MyApp::default()))
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! Connect via SSH:
//!
//! ```bash
//! ssh -p 2222 localhost
//! ```
//!
//! Press `Ctrl+C` to exit.

use crate::{backend::SSHUIBackend, ssh::SSHUIServer};
use anyhow::Result;
use ratatui::Terminal;
use russh::server::Server;
use std::{
    io::Write,
    sync::{Arc, atomic::AtomicUsize},
};
use tokio::net::ToSocketAddrs;

mod auth;
pub mod backend;
#[cfg(feature = "keyring")]
mod key;
mod lobby;
mod ssh;

#[cfg(feature = "keyring")]
pub use key::{get_debug_ssh_key, get_ssh_key};
pub type SSHUITerminal = Terminal<SSHUIBackend>;
pub use auth::{AuthDecision, AuthHandler, NoAuth};
pub use lobby::Lobby;
pub use ratatui;
pub use russh::server::Config;
pub use std::time::Duration;
pub use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};

/// The App trait that must be implemented for TUI applications served over SSH.
///
/// This trait defines the interface between the SSHUI framework and your Ratatui-based application.
/// Implement this trait to create an interactive TUI that can be accessed over SSH.
///
/// # Example
///
/// ```ignore
/// impl sshui::App for App {
///     fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>> {
///         terminal.draw(|frame| self.draw(frame))?;
///         
///         Ok(if self.exit {
///             Some("Exited".to_string())
///         } else {
///             None
///         })
///     }
///
///     fn input(&mut self, event: InputEvent) {
///         let InputEvent::Key(KeyEvent { key, .. }) = event else {
///             return;
///         };
///
///         match key {
///             // Handle input events
///             _ => {}
///         }
///     }
/// }
/// ```
pub trait App: Send + Sync {
    /// Renders the application to the terminal.
    ///
    /// This method is called whenever the terminal needs to be redrawn. It should draw the current
    /// application state using the provided ratatui Terminal.
    ///
    /// # Arguments
    ///
    /// * `terminal` - The ratatui terminal instance for drawing UI elements
    ///
    /// # Returns
    ///
    /// * `Ok(None)` if the application should continue running
    /// * `Ok(Some(message))` if the application should exit with the given message
    /// * `Err(e)` if a rendering error occurs
    fn render(&mut self, terminal: &mut SSHUITerminal) -> Result<Option<String>>;

    /// Handles user input events.
    ///
    /// This method is called whenever a user input event (keyboard, mouse, etc.) is received.
    /// Update your application state based on the input event.
    ///
    /// # Arguments
    ///
    /// * `event` - The input event to process
    fn input(&mut self, event: InputEvent);

    /// Called periodically for time-based updates (optional).
    ///
    /// This method is called at the refresh rate interval if configured.
    /// Use it for animations, progress bars, or other time-based updates.
    fn on_tick(&mut self) {
        // Default: no-op
    }

    /// Called when the front-end interface sends a message to it's backend
    fn on_message(&mut self) {
        // Default: no-op
    }
}

/// Starts an SSH server that serves your TUI application.
///
/// This function starts a russh SSH server on the specified address and begins accepting client
/// connections. Each connected client will get a fresh instance of your application created by
/// the provided factory function.
///
/// If you need to provide a config (auth, refresh_rate...), take a look at `new_server_with_config`.
///
/// # Arguments
///
/// * `server_config` - The SSH server configuration (see `russh::server::Config`)
/// * `addrs` - The address(es) to bind to (e.g., `("0.0.0.0", 2222)`)
/// * `app_factory` - A closure that creates a new instance of your App for each client connection
///
/// # Returns
///
/// * `Ok(())` if the server runs successfully
/// * `Err(e)` if the server fails to start or encounters an error
///
/// # Example
///
/// ```ignore
/// let config = sshui::Config::default();
/// sshui::new_server(config, ("0.0.0.0", 2222), || Box::new(App::default()))
///     .await
///     .unwrap();
/// ```
pub async fn new_server<
    A: ToSocketAddrs + Send + std::fmt::Debug,
    F: Fn() -> Box<dyn App> + Send + Sync + 'static,
>(
    server_config: Config,
    addrs: A,
    app_factory: F,
) -> Result<()> {
    new_server_with_config(server_config, addrs, app_factory, SSHUIConfig::default()).await
}

/// Starts an SSH server with the specified config
///
/// Same as `new_server` but a specified `SSHUIConfig` config.
///
/// # Arguments
///
/// * `server_config` - The SSH server configuration
/// * `addrs` - The address(es) to bind to
/// * `app_factory` - A closure that creates a new App instance for each client
/// * `config` - The SSHUIConfig
pub async fn new_server_with_config<
    A: ToSocketAddrs + Send + std::fmt::Debug,
    F: Fn() -> Box<dyn App> + Send + Sync + 'static,
>(
    server_config: Config,
    addrs: A,
    app_factory: F,
    config: SSHUIConfig,
) -> Result<()> {
    let server_config = Arc::new(server_config);
    let app_factory = Arc::new(app_factory);
    let mut server = SSHUIServer {
        app_factory,
        connected_clients: Arc::new(AtomicUsize::new(0)),
        auth: config.auth,
        refresh_rate: config.refresh_rate,
    };

    println!("Starting SSH server on {addrs:?}");
    print!("Waiting for clients...");
    std::io::stdout().flush()?;

    server.run_on_address(server_config, addrs).await?;

    Ok(())
}

/// A SSHUI server configuration struct.
///
/// # Fields
///
/// - `auth` (`Arc<dyn AuthHandler>`) - The AuthHandler that will decide if the client can connect or not.
/// - `refresh_rate` (`Option<Duration>`) - Optional refresh rate to re-render the app on a regular interval.
///
/// # Example
///
/// ```ignore
/// SSHUIConfig {
///     auth: Arc::new(MyAuth),
///     ..Default::default()
/// }
/// ```
pub struct SSHUIConfig {
    pub auth: Arc<dyn AuthHandler>,
    pub refresh_rate: Option<Duration>,
}

impl Default for SSHUIConfig {
    fn default() -> Self {
        Self {
            auth: Arc::new(NoAuth),
            refresh_rate: None,
        }
    }
}
