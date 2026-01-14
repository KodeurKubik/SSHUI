// Ported version of my Wordle
// Available under the MIT License at: https://github.com/KodeurKubik/wordle-rust

mod wordlist;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    let key_pair = sshui::get_debug_ssh_key()?;
    #[cfg(not(debug_assertions))]
    let key_pair = sshui::get_ssh_key()?;

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

    sshui::new_server(config, ("0.0.0.0", port), || {
        Box::new(wordle_ssh::WordleApp::default())
    })
    .await?;

    Ok(())
}
