# Better-IA: Internet Archive Explorer

A fast, native cross-platform desktop client for searching the Internet Archive (Archive.org) catalog and downloading complete media collections in parallel. Built from scratch in Rust with an asynchronous hardware-accelerated GUI.

![Platform Support](https://shields.io)
![License](https://shields.io)

---

## Features

* **True Async Performance:** The user interface runs at a fluid 60 FPS. All network requests run on an isolated background Tokio thread pipeline to ensure zero application freezes.
* **High-Speed Parallel Downloads:** Downloads utilize a multi-threaded `JoinSet` queue worker group. It streams media payload assets simultaneously to maximize your network bandwidth.
* **Persistent Search Memory:** Keeps track of your query inputs locally in an autonomous file map, generating a "Recent Searches" history dropdown that survives application restarts.
* **Live Throughput Speedometer:** Streams incoming buffers byte-by-byte, providing an instant calculation of your actual download speed in MB/s.
* **Download Cancellation Shield:** Allows you to instantly abort active parallel worker groups mid-transfer with a reactive Cancel button.
* **Advanced Catalog Filters:** Quickly toggle searches to filter between complete Collections or standard Single Files/Data objects.

---

## Installation & Compilation

### Prerequisites
Ensure you have the latest stable Rust toolchain installed. If not, get it from [rustup.rs](https://rustup.rs).

#### Linux Dependencies
If you are compiling on Linux, ensure your system has the standard development graphics headers installed:
```bash
# On Ubuntu/Debian/Pop!_OS:
sudo apt-get update
sudo apt-get install -y libwayland-dev libx11-dev libx11-xcb-dev libxkbcommon-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev libxrandr-dev libdbus-1-dev
```

### Building the Project
Clone the repository and compile the optimized standalone binary executable:

```bash
git clone https://github.com
cd Better-IA
cargo run --release
```
The compiled binary executable can be extracted directly from `target/release/`.

---

## Cross-Platform Distribution

This binary is safe to distribute standalone. It bundles its own secure TLS cryptography engines and root certificates (`webpki-roots`) and relies on an internal standalone DNS resolver (`hickory-dns`), making it completely independent of underlying host system network configurations.

### Cross-Compiling for Windows from Linux
If you want to build the Windows standalone `.exe` without leaving your Linux terminal:
```bash
# 1. Install the Windows compiler tools
sudo apt-get install mingw-w64
rustup target add x86_64-pc-windows-gnu

# 2. Build the Windows binary
cargo build --release --target x86_64-pc-windows-gnu
```
Your executable will be waiting at `target/x86_64-pc-windows-gnu/release/Better-IA.exe`.

---

## License

This project is licensed under the MIT License. See the LICENSE file for details.

---

## Contributing

Contributions, bug tracking issues, and feature additions are welcome. Feel free to fork the repository, open a pull request, or submit an issue tracker ticket.

