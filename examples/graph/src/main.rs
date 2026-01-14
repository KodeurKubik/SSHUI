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

    sshui::new_server(config, ("0.0.0.0", 2222), || {
        Box::new(graph_ssh::GraphApp::default())
    })
    .await?;

    Ok(())
}
