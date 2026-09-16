# `ast` — Android Studio TUI

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

A fast, lightweight, keyboard-driven Terminal User Interface (TUI) for Android development workflows. Built with **Rust**, **Ratatui**, and **Tokio**.

---

## ⚡ Features

### 1. Emulator & Device Management (`F1`)

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

### 2. Build & Gradle Sync (`F2`)

- **Dependency Syncing**: Run `./gradlew --refresh-dependencies help` to resolve and refresh project dependencies.
- **Debug Build**: Run `./gradlew assembleDebug --console=plain`. Outputs the APK directly to `./app/build/outputs/apk/debug/app-debug.apk` (identically to Android Studio) and displays the resulting file path and size.
- **Project Clean**: Run `./gradlew clean --console=plain`.
- **Cancel Ongoing Tasks**: Press `[x]` to terminate a running Gradle task immediately.
- **High-Performance Stream Viewport**:
    - Real-time syntax-highlighted stdout/stderr stream (tasks in cyan, success in bold green, errors in red).
    - Configurable auto-scrolling with `[a]` toggle and keyboard scrolling (`↑`/`↓`, `PgUp`/`PgDn`).
    - Clear output buffer with `[l]`.

### 3. Logs & Logcat (`F3`) _(Upcoming)_

- Real-time log streaming filtered by application package, PID, or log level.

---

## 🎮 Keybindings

### Global

| Key         | Action                               |
| ----------- | ------------------------------------ |
| `F1`        | Switch to **Emulator & Devices** Tab |
| `F2`        | Switch to **Build & Gradle** Tab     |
| `F3`        | Switch to **Logs & Logcat** Tab      |
| `q` / `Esc` | Quit `ast`                           |

### Emulator Tab (`F1`)

| Key               | Context         | Action                                                          |
| ----------------- | --------------- | --------------------------------------------------------------- |
| `Tab` / `←` / `→` | Any             | Switch focus between **Installed AVDs** and **Running Targets** |
| `↑` / `↓`         | Any             | Navigate items within the focused list                          |
| `Enter` / `r`     | Installed AVDs  | Launch selected AVD headlessly with host GPU                    |
| `s`               | Running Targets | Launch `scrcpy` display window _(Emulators only)_               |
| `b`               | Running Targets | Build Debug APK & deploy to selected target                     |
| `k`               | Running Targets | Stop selected emulator (`adb emu kill`)                         |
| `R`               | Any             | Force refresh device and AVD lists                              |

### Build Tab (`F2`)

| Key             | Action                                                   |
| --------------- | -------------------------------------------------------- |
| `s`             | Run **Gradle Sync** (`--refresh-dependencies help`)      |
| `b`             | Run **Assemble Debug** (`assembleDebug --console=plain`) |
| `c`             | Run **Clean Project** (`clean --console=plain`)          |
| `x`             | **Cancel / Stop** active Gradle task                     |
| `l`             | **Clear** output log buffer                              |
| `a`             | Toggle **Auto-scroll** (ON / OFF)                        |
| `↑` / `↓`       | Scroll output viewport by 1 line                         |
| `PgUp` / `PgDn` | Scroll output viewport by 15 lines                       |

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

Launch `ast` from inside any Android project root containing `./gradlew`:

```bash
cargo run
```

_(Or place the compiled binary in your `$PATH` and run `ast`)._

---

## 🏗️ Architecture & Philosophy

- **Asynchronous & Non-Blocking**: Heavy CLI processes (`adb`, `emulator`, `gradlew`, `scrcpy`) run on dedicated Tokio worker threads. UI redraws continue at 60fps without freezing or stuttering.
- **Sensible Device Handling**: Virtual emulators run headless to save resources and use `scrcpy` for display, while physical phones connected via USB are operated by hand.
- **Extensible Architecture**: Refer to [`AGENTS.md`](AGENTS.md) for full architectural guidelines, crate layout, and CLI integration specs.

---

## 📜 License

This project is licensed under the **GNU General Public License v3.0** (GPLv3). See the [`LICENSE`](LICENSE) file for details.
