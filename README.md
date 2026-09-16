# `ast` — Android Studio TUI

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

A fast, lightweight, keyboard-driven Terminal User Interface (TUI) for Android development workflows. Built with **Rust**, **Ratatui**, and **Tokio**.

---

## ⚡ Features

### 1. Emulator & Device Management (F1)
- **Automatic Device Discovery**:
  - Lists installed Android Virtual Devices (AVDs) via `emulator -list-avds`.
  - Lists active USB-connected physical devices and running emulators via `adb devices -l`.
  - Automatic background polling every 2.5s with non-blocking UI.
- **Headless Virtual Device Execution**:
  - Launch AVDs headlessly with host GPU acceleration (`-no-window -no-audio -no-boot-anim -gpu host`) for high performance and low memory footprint.
- **Integrated Display & Controls via `scrcpy`**:
  - One-key display mirroring for emulators (`scrcpy --mouse=sdk --keyboard=sdk`).
  - Differentiates physical USB phones (operated by hand) from virtual emulators.
- **One-Key Build & Deploy**:
  - Build debug APKs (`./gradlew assembleDebug --console=plain`) and install directly to the selected target (`./gradlew installDebug`).
- **Live Output Drawer**:
  - Real-time rolling command logs showing Gradle build output, boot statuses, and process notifications.

### 2. Build & Gradle (F2) *(Upcoming)*
- Run common Gradle tasks, view dependency trees, and inspect build artifacts.

### 3. Logs & Logcat (F3) *(Upcoming)*
- Real-time log streaming filtered by application package, PID, or log level.

---

## 🎮 Keybindings

### Global
| Key | Action |
| --- | --- |
| `F1` | Switch to **Emulator & Devices** Tab |
| `F2` | Switch to **Build & Gradle** Tab |
| `F3` | Switch to **Logs & Logcat** Tab |
| `q` / `Esc` | Quit `ast` |

### Emulator Tab (`F1`)
| Key | Context | Action |
| --- | --- | --- |
| `Tab` / `←` / `→` | Any | Switch focus between **Installed AVDs** and **Running Targets** |
| `↑` / `↓` | Any | Navigate items within the focused list |
| `Enter` / `r` | Installed AVDs | Launch selected AVD headlessly with host GPU |
| `s` | Running Targets | Launch `scrcpy` display window *(Emulators only)* |
| `b` | Running Targets | Build Debug APK & deploy to selected target |
| `k` | Running Targets | Stop selected emulator (`adb emu kill`) |
| `R` | Any | Force refresh device and AVD lists |

---

## 🛠️ Prerequisites

Ensure the following tools are installed and available in your `PATH`:
- **Rust** (2024 edition / 1.85+)
- **Android SDK Platform-Tools** (`adb`)
- **Android SDK Emulator** (`emulator`)
- **scrcpy** (optional, recommended for controlling headless emulators)
- **JDK 17+** (for Gradle builds)

Verify standard tool locations:
```bash
which adb emulator scrcpy
```

---

## 🚀 Getting Started

### Clone & Build
```bash
git clone https://github.com/your-username/ast.git
cd ast
cargo build --release
```

### Run
Launch `ast` from inside any Android project root:
```bash
cargo run
```
*(Or place the compiled binary in your `$PATH` and run `ast`).*

---

## 🏗️ Architecture & Philosophy

- **Asynchronous & Non-Blocking**: Heavy CLI processes (`adb`, `emulator`, `gradlew`, `scrcpy`) run on dedicated Tokio worker threads. UI redraws continue at 60fps without freezing or stuttering.
- **Sensible Device Handling**: Virtual emulators run headless to save resources and use `scrcpy` for display, while physical phones connected via USB are operated by hand.
- **Extensible Architecture**: Refer to [`AGENTS.md`](AGENTS.md) for full architectural guidelines, crate layout, and CLI integration specs.

---

## 📜 License

Dual-licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.
