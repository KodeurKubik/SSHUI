cargo build -p example --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/example build/sshui-linux
upx --best --lzma build/sshui-linux