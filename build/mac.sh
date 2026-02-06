cargo build -p example --release
cp target/release/example build/sshui-mac
upx --best --lzma build/sshui-mac