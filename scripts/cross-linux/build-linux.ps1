$crossDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:PATH = "$crossDir;$env:PATH"
$env:CC_x86_64_unknown_linux_gnu = 'x86_64-linux-gnu-gcc'
$env:AR_x86_64_unknown_linux_gnu = 'x86_64-linux-gnu-ar'
$env:CFLAGS_x86_64_unknown_linux_gnu = '--target=x86_64-unknown-linux-gnu'
cargo build --release -p qualia-cli --target x86_64-unknown-linux-gnu
Write-Host 'Linux binary should be in target/x86_64-unknown-linux-gnu/release/qualia-cli (relative to project root)'