# AGENTS.md — Agent & Developer Guide for `ast` (Android Studio TUI)

## 1. Project Overview

`ast` (Android Studio TUI) is a lightweight, responsive terminal user interface designed as a fast keyboard-driven companion/alternative to Android Studio for day-to-day Android development workflows.

The project is structured around three primary functional tabs:

1. **Emulator & Devices (F1)**: Discovery, launching, controlling virtual devices (AVDs), USB-connected physical devices, scrcpy mirroring for virtual devices, and one-key build & deploy with auto-launch.
2. **Build & Gradle (F2)**: Dependency syncing, debug builds (`assembleDebug`), clean tasks, cancellation, and syntax-highlighted build output streaming.
3. **Logs & Logcat (F3)**: Real-time multi-device application log streaming, disk persistence under `~/.ast/logs/`, package/PID filtering, and search navigation.

---

## 2. Technology Stack & Crates

- **Language**: Rust (2024 edition)
- **UI Framework**: [`ratatui`](https://crates.io/crates/ratatui) (v0.30+)
- **Terminal Backend**: [`crossterm`](https://crates.io/crates/crossterm) (v0.29+)
- **Async Runtime**: [`tokio`](https://crates.io/crates/tokio) (v1.53+) — for non-blocking sub-processes (`adb`, `emulator`, `gradlew`, `scrcpy`)
- **CLI Parsing**: [`clap`](https://crates.io/crates/clap) (v4.6+, derive) — for CLI options (`-p / --project-path`)
- **Error Handling**: [`color-eyre`](https://crates.io/crates/color-eyre)

---

## 3. Architecture & Design Principles

### Non-Blocking Event Loop & Channel Backpressure

**Rule 1: Never execute blocking subprocesses on the main UI thread.**

- Ratatui's event loop handles terminal redraws and keyboard inputs (`crossterm::event::EventStream`).
- Background operations (polling `adb devices`, querying `sys.boot_completed`, launching `emulator`, running `./gradlew`, streaming `adb logcat`) MUST be executed asynchronously using `tokio::spawn` or background worker channels (`tokio::sync::mpsc`).
- High-throughput streams (such as `logcat` producing hundreds of lines/second) use a generously sized channel (`mpsc::channel(2000)`) and are drained in batches in `main.rs` to avoid blocking the sender and prevent unnecessary redraw loops.
- Communication between background tasks and the UI loop flows via the `AppEvent` channel (e.g. `AppEvent::DevicesRefreshed`, `AppEvent::LogcatLine`, `AppEvent::AppPidResolved`).

### Context-Aware Key Mapping & Action Routing

**Rule 2: Keep `main.rs` minimal. Isolate tab key-bindings and executions in `src/handler/<tab>.rs`.**

- In `main.rs`, only global shortcuts (`q`, `Esc`, `F1`–`F3`) and search modal input routing are handled globally.
- All tab-specific keys are delegated to the active tab's handler module using a two-stage pattern:
    1. **Pure Key Mapping**: `(KeyCode, State/Mode) -> Option<TabAction>`
    2. **Action Dispatcher**: `execute_action(action, state, tx)`
- Dedicated handlers:
    - [`src/handler/emulator.rs`](src/handler/emulator.rs)
    - [`src/handler/build.rs`](src/handler/build.rs)
    - [`src/handler/logs.rs`](src/handler/logs.rs)

### Project Root & Working Directory Management

**Rule 3: All Gradle operations (`./gradlew`) execute relative to the configured project root.**

- `ast` accepts an optional CLI flag: `-p` / `--project-path <DIR>`.
- At startup, `ast` validates that the path exists and is a directory, canonicalizes it, switches the process working directory via `std::env::set_current_dir`, and stores it in `AppState.project_dir`.
- Gradle tasks and APK lookup logic (`app/build/outputs/apk/debug/app-debug.apk`) execute relative to this root.

### Logcat Management & Multi-Device Isolation

**Rule 4: Keep logcat sessions distinct, persistent, and bounded.**

- **Multi-Device Tabs**: `DeviceLogSession` tracks serial, display name, in-memory lines (up to 10,000 with oldest-drain), scroll position, search query, and package PID.
- **Disk Persistence**: Streaming logs append to `~/.ast/logs/<serial>.log`. On startup or device discovery, existing sessions pre-hydrate their recent 1,000 lines from disk.
- **Tailing Stream**: Spawning `adb -s <serial> logcat -v time -T 500` ensures initial logs start with recent events instead of loading hundreds of thousands of historical log lines from device flash.
- **Dynamic PID Tracking**: Applications often launch asynchronously. `build_and_deploy_apk` polls `adb shell pidof <package>` and dispatches `AppEvent::AppPidResolved`, enabling instantaneous filtering on `Tag(PID):` and threadtime signatures.

### Device Interaction Rules

- **Virtual Emulators**: Run headless (`-no-window -no-audio -no-boot-anim -gpu host`). Use `scrcpy` to display their screen and control them with mouse and keyboard (`--mouse=sdk --keyboard=sdk`).
- **Physical USB Devices**: Used by hand directly. Screen mirroring via `scrcpy` is **not** required and should not be offered or triggered for physical devices.

### Module Structure

```
src/
├── main.rs            # Entrypoint, CLI args, terminal initialization, async event loop & line batching
├── model.rs           # Core domain models (AvdInfo, DeviceTarget, DeviceLogSession, LogState, GradleState)
├── events.rs          # AppEvent enum (DevicesRefreshed, LogcatLine, AppPidResolved, GradleTaskFinished, etc.)
├── app.rs             # Application state, log session management, device tracking, scroll operations
├── handler/
│   ├── mod.rs         # Handlers module declaration
│   ├── emulator.rs    # Emulator tab key mapping & action execution
│   ├── build.rs       # Build tab key mapping & action execution
│   └── logs.rs        # Logs tab key mapping (normal/edit modes) & action execution
├── ui/
│   ├── mod.rs         # Root layout (header, project label, tab bar, footer status)
│   ├── emulator.rs    # Emulator & device management view (dual pane + drawer)
│   ├── build.rs       # Gradle build & sync view (viewport, status banner, syntax highlight)
│   └── logs.rs        # Device log tabs, search & filter bar, syntax-highlighted logcat viewport
└── android/
    ├── mod.rs         # Android subsystem coordinator
    ├── discovery.rs   # Virtual device discovery & ADB device parsing
    ├── lifecycle.rs   # Headless emulator start, boot status polling, scrcpy, kill
    ├── deploy.rs      # Package name detection, build/install, launch_app_on_target, get_package_pid
    ├── logcat.rs      # Multi-device streaming manager, stop/start channels, file persistence
    └── gradle.rs      # Gradle async runner, PID tracking, and cancellation
```

---

## 4. Android Tooling Integration Reference

### A. Device Discovery

- **Installed AVDs**:
    ```bash
    emulator -list-avds
    ```
- **Active Devices (USB & Running Emulators)**:
    ```bash
    adb devices -l
    ```

### B. Virtual Device Lifecycle

- **Launch Headless (with Host GPU acceleration)**:
    ```bash
    emulator -avd <AVD_NAME> -no-window -no-audio -no-boot-anim -gpu host
    ```
- **Check Boot Completion**:
    ```bash
    adb -s <SERIAL> shell getprop sys.boot_completed
    ```
- **Stop / Kill Emulator**:
    ```bash
    adb -s <SERIAL> emu kill
    ```

### C. Scrcpy Display Mirroring (Virtual Devices Only)

- **Launch Display Client for Emulator**:
    ```bash
    scrcpy -s <SERIAL> --window-title "<TITLE>" --stay-awake --mouse=sdk --keyboard=sdk
    ```

### D. Build & Deployment

- **Dependency Sync**:
    ```bash
    ./gradlew --refresh-dependencies help
    ```
- **Build Debug APK**:
    ```bash
    ./gradlew assembleDebug --console=plain
    ```
- **Install Debug APK**:
    ```bash
    ./gradlew installDebug --console=plain
    ```
- **Resolve Launcher & Launch App**:
    ```bash
    adb -s <SERIAL> shell cmd package resolve-activity --brief <PACKAGE>
    adb -s <SERIAL> shell am start -n <PACKAGE>/<ACTIVITY> -a android.intent.action.MAIN -c android.intent.category.LAUNCHER
    ```
    _Fallback_:
    ```bash
    adb -s <SERIAL> shell monkey -p <PACKAGE> -c android.intent.category.LAUNCHER 1
    ```
- **Query Installed Version**:
    ```bash
    adb -s <SERIAL> shell dumpsys package <PACKAGE>
    ```

### E. Logcat & PID Resolution

- **Resolve Package PID**:
    ```bash
    adb -s <SERIAL> shell pidof <PACKAGE>
    ```
- **Stream Logs**:
    ```bash
    adb -s <SERIAL> logcat -v time -T 500
    ```
    Piped asynchronously to `~/.ast/logs/<serial>.log` and batched to the TUI event channel.

---

## 5. Development & Testing Instructions

- **Build**: `cargo build`
- **Release Build**: `cargo build --release`
- **Run Locally**: `cargo run -- -p /path/to/android/project`
- **Install Globally**: `cargo install --path .` (installs to `~/.cargo/bin/ast`)
- **Check**: `cargo check` / `cargo clippy`
- **Test**: `cargo test` (all 20+ unit tests cover discovery, gradle scrolling, log sessions, logcat filtering, and PID resolution)
