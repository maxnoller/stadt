# Stadt

A Bevy game project.

## Prerequisites

### Rust
Install Rust: https://rustup.rs/

### Linux Dependencies
You need to install the following development libraries:

**Ubuntu/Debian:**
```bash
sudo apt-get install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0 libwayland-dev libxkbcommon-dev
```

**Fedora:**
```bash
sudo dnf install gcc-c++ libX11-devel alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel
```

**Arch/Manjaro:**
```bash
sudo pacman -S libx11 pkgconf alsa-lib libxcursor libxrandr libxi
```

## Running the project

```bash
cargo run
```

## Development

- **Formatting**: Code is automatically formatted on commit hooks.
- **Linting**: Run `cargo clippy` to catch common mistakes.
- **Testing**: Run `cargo test` to execute tests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
