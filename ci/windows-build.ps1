invoke-restmethod -usebasicparsing 'https://static.rust-lang.org/rustup/dist/i686-pc-windows-gnu/rustup-init.exe' -outfile 'rustup-init.exe'
invoke-restmethod -usebasicparsing 'https://github.com/ethankhall/crom/releases/download/v0.1.9/crom-windows.zip' -outfile 'crom-windows.zip'
Expand-Archive -LiteralPath crom-windows.zip
./crom-windows/crom.exe update-version --pre-release release
./rustup-init.exe -y --default-toolchain nightly-x86_64-pc-windows-msvc --no-modify-path
& "$env:USERPROFILE/.cargo/bin/rustup.exe" install nightly-x86_64-pc-windows-msvc
remove-item rustup-init.exe
& "$env:USERPROFILE/.cargo/bin/cargo.exe" +nightly-x86_64-pc-windows-msvc test
& "$env:USERPROFILE/.cargo/bin/cargo.exe" +nightly-x86_64-pc-windows-msvc build --release