// Ported version of the Ratatui Demo, available under the MIT License (see ../LICENSE)
// and available at: https://github.com/ratatui/ratatui/tree/main/examples/apps/demo

//! # [Ratatui] Original Demo example
//!
//! [Ratatui]: https://github.com/ratatui/ratatui
//! [examples]: https://github.com/ratatui/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui/ratatui/blob/main/examples/README.md

use sshui::SSHUIConfig;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    #[cfg(debug_assertions)]
    let key_pair = sshui::get_debug_ssh_key().unwrap();
    #[cfg(not(debug_assertions))]
    let key_pair = sshui::get_ssh_key().unwrap();

    let config = sshui::Config {
        keys: vec![key_pair],
        ..Default::default()
    };

    let mut port = 2222u16;
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            port = args[i + 1].parse().unwrap_or(2222);
            break;
        }
    }

    sshui::new_server_with_config(
        config,
        ("0.0.0.0", port),
        || Box::new(demo_ssh::App::new(demo_ssh::ENHANCED_GRAPHICS)),
        SSHUIConfig {
            refresh_rate: Some(demo_ssh::REFRESH_RATE),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    Ok(())
}
