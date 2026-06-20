set -e
cargo build --release
install -Dm755 target/release/mx-gesture-daemon ~/.local/bin/mx-gesture-daemon
sudo install -m644 42-logitech-hidpp.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
#sudo usermod -aG plugdev "$USER"   # then log out + in
mkdir -p ~/.config/mx-gesture-daemon
cp config.example.toml ~/.config/mx-gesture-daemon/config.toml
systemctl --user restart mx-gesture-daemon
#$EDITOR ~/.config/mx-gesture-daemon/config.toml