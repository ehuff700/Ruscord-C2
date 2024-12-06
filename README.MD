# Ruscord C2

A Command and Control (C2) framework written in Rust, using Discord infrastructure as a communication channel. Ruscord C2 is designed to be a flexible and powerful tool for managing and controlling systems remotely through Discord. It is intended to be used for authorized testing and development purposes only. It is a project not designed with OPSEC in mind, and is likely easily detectable and blockable. 

## Prerequisites

- Rust 1.70 or higher + Cargo
- Discord bot token with appropriate permissions (see [Discord Developer Portal](https://discord.com/developers/docs/quick-start/getting-started))
- A Guild (Discord Server) for the bot to connect to. [Turn on developer mode](https://www.howtogeek.com/714348/how-to-enable-or-disable-developer-mode-on-discord/) to access the Guild ID, required for `ruscord_c2.toml`
- Windows 10/11 tested, Linux and MacOS supported, but untested.


## Building

```bash
# Clone the repository
git clone https://github.com/ehuff700/ruscord_c2.git
cd ruscord_c2

# Build the project
cargo build --release
```

## Configuration

See `ruscord_example.toml` for configuration options. 

## Usage

```bash
# Run the bot
cargo run --release
```

### Available Commands
- `/help` - Display available commands and usage
- `/config` - View current configuration
- `/sysinfo` - Display system information
- `/tunnel` - Create a reverse TCP shell to a remote host.
- `/ps` - List running processes.
- `/ps spawn` - Spawn a new process.
- `/ps kill` - Kill a running process.
- `/ls` - List directory contents.
- `/cd` - Change working directory.
- `/upload`/`/download` - File transfer operations.
- `/screen` - Take screenshots, record video from screen.
- `/clipboard` - Access clipboard
- And many more...

## Security Notice

This tool is for educational and authorized testing purposes only. Always ensure you have proper authorization before using any C2 framework. The authors are not responsible for any misuse or damage caused by this software.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
