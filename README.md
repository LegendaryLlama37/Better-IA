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

## Installation & Download

### Pre-compiled Binaries (Recommended)
You do not need to compile this software from source. Ready-to-run desktop applications for both Windows and Linux are automatically built and published for every release.

1. Navigate to the Releases section on the right side of this GitHub repository page.
2. Download the version corresponding to your operating system:
   * For Windows: Download `better_ia_windows.exe` (Run the standalone executable directly).
   * For Linux: Download `better_ia_linux` (Mark as executable via `chmod +x better_ia_linux` and launch).

---

## Building from Source (Developers Only)

If you wish to modify the codebase or build the binary manually, follow the development compilation steps below.

### Prerequisites
Ensure you have the latest stable Rust toolchain installed. If not, get it from [rustup.rs](https://rustup.rs).

#### Linux System Dependencies
Compiling the hardware-accelerated GUI graphics stack natively on Linux requires standard development graphics headers:
```bash
# On Ubuntu/Debian/Pop!_OS:
sudo apt-get update
sudo apt-get install -y libwayland-dev libx11-dev libx11-xcb-dev libxkbcommon-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev libxrandr-dev libdbus-1-dev
```

### Compiling Natively
Clone the repository and build the optimized standalone production release binary:

```bash
git clone https://github.com
cd Better-IA
cargo run --release
```
The newly generated executable can be extracted directly from `target/release/`.

---

## Cross-Platform Design

This binary is safe to distribute standalone. It bundles its own secure TLS cryptography engines and root certificates (`webpki-roots`) and relies on an internal standalone DNS resolver (`hickory-dns`), making it completely independent of underlying host system network configurations.

---

## License

This project is licensed under the MIT License. See the LICENSE file for details.

---

## Contributing

Contributions, bug tracking issues, and feature additions are welcome. Feel free to fork the repository, open a pull request, or submit an issue tracker ticket.
