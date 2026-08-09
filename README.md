<h1 align="center">Rust Shakes & Fidget Bot</h1>

<h4 align="center">
  <a href="#features">Features</a>
  ·
  <a href="#download">Download</a>
  ·
  <a href="#compilation">Compilation</a>
  ·
  <a href="#usage">Usage</a>
</h4>

<p align="center">
    <a href="https://github.com/tjira/rsfb/pulse">
        <img src="https://img.shields.io/github/last-commit/tjira/rsfb?style=for-the-badge"/>
    </a>
    <a href="https://github.com/tjira/rsfb/blob/master/LICENSE">
        <img src="https://img.shields.io/github/license/tjira/rsfb?style=for-the-badge"/>
    </a>
    <a href="https://github.com/tjira/rsfb/actions/workflows/release.yml">
        <img src="https://img.shields.io/github/actions/workflow/status/tjira/rsfb/release.yml?style=for-the-badge&label=release"/>
    </a>
    <br>
    <a href="https://github.com/tjira/rsfb">
        <img src="https://img.shields.io/github/languages/code-size/tjira/rsfb?style=for-the-badge"/>
    </a>
    <a href="https://github.com/tjira/rsfb">
        <img src="https://img.shields.io/endpoint?url=https://ghloc.vercel.app/api/tjira/rsfb/badge?filter=.rs&style=for-the-badge&format=human"/>
    </a>
    <a href="https://github.com/tjira/rsfb/stargazers">
        <img src="https://img.shields.io/github/stars/tjira/rsfb?style=for-the-badge"/>
    </a>
    <a href="https://github.com/tjira/rsfb/releases/latest">
        <img src="https://img.shields.io/github/downloads/tjira/rsfb/total?style=for-the-badge"/>
    </a>
    <br>
</p>

<p align="center">
Lightweight asynchronous Rust bot for Shakes & Fidget automation. Designed to be robust and parallel-friendly, it manages multiple character sessions simultaneously, features human-like delay simulation, and automates daily activities, fortress upgrades, and underworld activities.
</p>

## Download

Pre-compiled binaries for various platforms are available on the [Releases](https://github.com/tjira/rsfb/releases) page.

## Compilation

To compile `rsfb` from source, you need to have the Rust toolchain (including `cargo`) installed. If you don't have it yet, you can install it from [rustup.rs](https://rustup.rs/).

1. Clone the repository:
    ```bash
    git clone https://github.com/tjira/rsfb.git
    ```

2. Navigate to the project directory:
    ```bash
    cd rsfb
    ```

3. Build the project in release mode:
    ```bash
    cargo build --release
    ```

4. The compiled executable will be available at:
   * **Linux/macOS**: `target/release/rsfb`
   * **Windows**: `target/release/rsfb.exe`

## Usage

Run the compiled binary from your terminal by passing your Shakes & Fidget account credentials:

```bash
rsfb <USERNAME> <PASSWORD>
```

Upon launching, the bot will log into your account, spawn individual async threads for each of your characters, and periodically print a status table in your terminal:

```text
+----------------+-------+--------------+------------+-----------+----------+--------------+
| CHARACTER NAME | LEVEL | CLASS        | GOLD       | MUSHROOMS | RANK     | STATUS       |
+----------------+-------+--------------+------------+-----------+----------+--------------+
| Hero One       |   385 | Demon Hunter |  412589.50 |       320 |     1420 | IDLE         |
| Hero Two       |   210 | Mage         |   12450.25 |        45 |     8942 | EXPEDITION   |
| Hero Three     |   124 | Warrior      |    3402.10 |        12 |    15201 | WORKING (8H) |
+----------------+-------+--------------+------------+-----------+----------+--------------+
```

## Credits

This project relies on the following libraries:

* [sf-api](https://github.com/the-marenga/sf-api) - An API wrapper/library for Shakes & Fidget.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
