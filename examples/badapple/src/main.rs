pub const TICK_RATE: std::time::Duration = std::time::Duration::from_millis(66);

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

    sshui::new_server_with_refresh(config, ("0.0.0.0", port), TICK_RATE, || {
        Box::new(badapple_ssh::App::default())
    })
    .await
    .unwrap();

    Ok(())
}
