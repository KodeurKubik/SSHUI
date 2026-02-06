cargo build -p example --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/example.exe build/sshui-win.exe
upx --best --lzma build/sshui-win.exe