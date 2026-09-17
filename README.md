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
- **One-Key Build, Deploy & Auto-Launch**:
    - Build debug APKs (`./gradlew assembleDebug --console=plain`) and install directly to the selected target (`./gradlew installDebug`).
    - Automatically discovers package name (`namespace` or `applicationId` from Gradle scripts / `AndroidManifest.xml`).
    - Resolves default launcher activity via `cmd package resolve-activity` or monkey launcher and launches the application on the target immediately upon deployment.
    - Automatically transitions to the **Logs Tab (`F3`)** and attaches live application logcat streaming.
- **Live Output Drawer**:
    - Real-time rolling command logs showing Gradle build output, boot statuses, and process notifications.

### 2. Build & Gradle Sync (`F2`)

- **Dependency Syncing**: Run `./gradlew --refresh-dependencies help` to resolve and refresh project dependencies.
- **Debug Build**: Run `./gradlew assembleDebug --console=plain`. Outputs the APK directly to `./app/build/outputs/apk/debug/app-debug.apk` (identically to Android Studio) and displays the resulting file path and size.
- **Project Clean**: Run `./gradlew clean --console=plain`.\
- **Cancel Ongoing Tasks**: Press `[x]` to terminate a running Gradle task immediately.
- **High-Performance Stream Viewport**:
    - Real-time syntax-highlighted stdout/stderr stream (tasks in cyan, success in bold green, errors in red).
    - Configurable auto-scrolling with `[a]` toggle and keyboard scrolling (`↑`/`↓`, `PgUp`/`PgDn`).
    - Clear output buffer with `[l]`.

### 3. Multi-Device Logs & Logcat (`F3`)

- **Multi-Device Tab Switching**:
    - Each connected emulator or physical device has an independent log stream and session.
    - Switch active device logs effortlessly using `[` / `]` or number keys `1`–`9`.
    - Distinct streaming status indicators (● green streaming, ○ gray paused) per device.
- **Persistent Log Storage (`~/.ast/logs/`)**:
    - All logcat streams are persisted to `~/.ast/logs/<serial>.log`.
    - Logs persist across device switches and application restarts, automatically hydrating recent logs on startup.
- **Application Process & Package Filtering**:
    - Real-time package filtering via `package:<package.name>` (e.g. `package:com.example.flocky`).
    - Dynamically resolves running app PID (`adb shell pidof <package>`) with asynchronous polling and displays an attached process indicator `[PID: <pid>]`.
    - Matches log lines by package name and Android logcat PID signatures (`Tag(PID):`, `Tag( PID):`, and space-padded threadtime PIDs).
- **Search & Filter Bar**:
    - Press `/` to enter filter edit mode; type any tag, error keyword, or `package:<name>`.
    - Press `Enter` to apply filter or `Esc` to cancel.
    - Press `c` to instantly clear the active filter and view raw device logs.
- **High-Throughput Log Streaming**:
    - Tail streams with `-T 500` to prevent buffer overflows from historic system logs.
    - Batch event draining and ring-buffered line retention (up to 10,000 lines per device session).
    - Auto-scroll lock toggling with `a`, `↑`/`↓`, `PgUp`/`PgDn`, `g` (top), and `G` (bottom).

---

## 🎮 Keybindings

### Global

| Key | Action |
| ----------- | ------------------------------------ |\
| `F1` | Switch to **Emulator & Devices** Tab |
| `F2` | Switch to **Build & Gradle** Tab |
| `F3` | Switch to **Logs & Logcat** Tab |
| `q` / `Esc` | Quit `ast` |

### Emulator Tab (`F1`)

| Key | Context | Action |\
| ----------------- | --------------- | --------------------------------------------------------------- |\
| `Tab` / `←` / `→` | Any | Switch focus between **Installed AVDs** and **Running Targets** |\
| `↑` / `↓` | Any | Navigate items within the focused list |\
| `Enter` / `r` | Installed AVDs | Launch selected AVD headlessly with host GPU |\
| `s` | Running Targets | Launch `scrcpy` display window _(Emulators only)_ |\
| `b` | Running Targets | Build Debug APK, deploy, auto-launch & stream logs |\
| `k` | Running Targets | Stop selected emulator (`adb emu kill`) |\
| `R` | Any | Force refresh device and AVD lists |

### Build Tab (`F2`)

| Key | Action |\
| --------------- | -------------------------------------------------------- |\
| `s` | Run **Gradle Sync** (`--refresh-dependencies help`) |\
| `b` | Run **Assemble Debug** (`assembleDebug --console=plain`) |\
| `c` | Run **Clean Project** (`clean --console=plain`) |\
| `x` | **Cancel / Stop** active Gradle task |\
| `l` | **Clear** output log buffer |\
| `a` | Toggle **Auto-scroll** (ON / OFF) |\
| `↑` / `↓` | Scroll output viewport by 1 line |\
| `PgUp` / `PgDn` | Scroll output viewport by 15 lines |

### Logs Tab (`F3`)

| Key | Mode | Action |\
| ----------------- | ------------- | --------------------------------------------------------- |\
| `[` / `]` | Normal | Switch to Previous / Next device log stream |\
| `1`–`9` | Normal | Select device tab 1–9 directly |\
| `/` | Normal | Focus search bar (`package:<name>` or keyword filter) |\
| `c` | Normal | Clear search filter (show all logs) |\
| `s` | Normal | Toggle logcat streaming (Start / Pause) |\
| `a` | Normal | Toggle auto-scroll |\
| `l` | Normal | Clear in-memory log buffer for active device |\
| `↑` / `↓` | Normal | Scroll log view by 1 line |\
| `PgUp` / `PgDn` | Normal | Scroll log view by 15 lines |\
| `g` / `Home` | Normal | Jump to top of log stream |\
| `G` / `End` | Normal | Jump to bottom of log stream |\
| `Enter` | Search/Filter | Apply filter query |\
| `Esc` | Search/Filter | Cancel editing / exit search bar |

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

## 🚀 Installation & Getting Started

### 1. Install Globally (Recommended)

Install the binary directly into `~/.cargo/bin`:

```bash
git clone https://github.com/your-username/ast.git
cd ast
cargo install --path .
```

### 2. Usage in Android Projects

You can run `ast` in two ways:

#### A. From inside your Android project root:

```bash
cd /path/to/your/android/project
ast
```

#### B. From anywhere by passing the project path (`-p` / `--project-path`):

```bash
ast -p /path/to/your/android/project
# or
ast --project-path /home/user/AndroidStudioProjects/MyAwesomeApp
```

### CLI Options

```
Usage: ast [OPTIONS]

Options:
  -p, --project-path <PROJECT_PATH>  Path to Android project root containing gradlew [default: .]
  -h, --help                         Print help
  -V, --version                      Print version
```

---

## 🏗️ Architecture & Philosophy

- **Asynchronous & Non-Blocking**: Heavy CLI processes (`adb`, `emulator`, `gradlew`, `scrcpy`) run on dedicated Tokio worker threads. UI redraws continue smoothly without freezing or stuttering.
- **Sensible Device Handling**: Virtual emulators run headless to save resources and use `scrcpy` for display, while physical phones connected via USB are operated by hand.
- **Robust Multi-Device Logcat**: Tail limits (`-T 500`) prevent system backlog choking, disk persistence under `~/.ast/logs/` guarantees history across sessions, and dynamic PID tracking keeps application logs isolated and clean.
- **Extensible Architecture**: Refer to [`AGENTS.md`](AGENTS.md) for full architectural guidelines, crate layout, and CLI integration specs.

---

## 📜 License

This project is licensed under the **GNU General Public License v3.0** (GPLv3). See the [`LICENSE`](LICENSE) file for details.
