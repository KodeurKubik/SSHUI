use std::time::Duration;

use chat_ssh::Message;
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

    let lobby = sshui::Lobby::<Message>::new(100)
        .with_validator(|msg| msg.content.len() <= 200 && msg.author.len() <= 25);

    sshui::new_server_with_config(
        config,
        ("0.0.0.0", port),
        move || Box::new(chat_ssh::App::new(lobby.clone())),
        SSHUIConfig {
            refresh_rate: Some(Duration::from_millis(500)),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    Ok(())
}
