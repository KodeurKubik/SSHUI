// Ported version of Ratatui's Basic Counter App
// Available under the MIT License at: https://github.com/ratatui/ratatui-website/blob/main/code/tutorials/counter-app-basic/src/main.rs

use std::io;

// This function changes quite a lot
// instead of running the terminal directly,
// run the server with a on-client-accept closure
// and it has to go async
#[tokio::main]
async fn main() -> io::Result<()> {
    #[cfg(debug_assertions)]
    let key_pair = sshui::get_debug_ssh_key().unwrap();
    #[cfg(not(debug_assertions))]
    let key_pair = sshui::get_ssh_key().unwrap();

    let config = sshui::Config {
        keys: vec![key_pair],
        ..Default::default()
    };

    sshui::new_server(config, ("0.0.0.0", 2222), || {
        Box::new(counter_ssh::App::default())
    })
    .await
    .unwrap();

    Ok(())
}

// You can find the remaining code in the `lib.rs` file in the same folder
