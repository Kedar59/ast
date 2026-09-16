# AGENTS.md — Agent & Developer Guide for `ast` (Android Studio TUI)

## 1. Project Overview

`ast` (Android Studio TUI) is a lightweight, responsive terminal user interface designed as a fast keyboard-driven companion/alternative to Android Studio for day-to-day Android development workflows.

The project is structured around three primary functional tabs:

1. **Emulator & Devices (F1)**: Discovery, launching, controlling virtual devices (AVDs), USB-connected physical devices, and scrcpy mirroring for virtual devices.
2. **Build & Gradle (F2)**: Dependency syncing, debug/release builds, Gradle tasks, and build output inspection.
3. **Logs & Logcat (F3)**: Real-time application log streaming filtered by package name, PID, or log level.

---

## 2. Technology Stack & Crates

- **Language**: Rust (2024 edition)
- **UI Framework**: [`ratatui`](https://crates.io/crates/ratatui) (v0.30+)
- **Terminal Backend**: [`crossterm`](https://crates.io/crates/crossterm) (v0.29+)
- **Async Runtime**: [`tokio`](https://crates.io/crates/tokio) (v1.53+) — for non-blocking sub-processes (`adb`, `emulator`, `gradlew`, `scrcpy`)
- **Embedded Terminal Widget**: [`tui-term`](https://crates.io/crates/tui-term) — for streaming command output and logs
- **Error Handling**: [`color-eyre`](https://crates.io/crates/color-eyre)

---

## 3. Architecture & Design Principles

### Non-Blocking Event Loop

**Rule 1: Never execute blocking subprocesses on the main UI thread.**

- Ratatui's event loop handles terminal redraws and keyboard inputs (`100ms` poll or event-driven).
- Background operations (polling `adb devices`, querying `sys.boot_completed`, launching `emulator`, running `./gradlew`) MUST be executed asynchronously using `tokio::spawn` or background worker channels (`tokio::sync::mpsc`).
- Communication between background tasks and the UI loop flows via an `AppEvent` channel (e.g. `AppEvent::DevicesRefreshed(Vec<Device>)`, `AppEvent::ActionProgress(String)`).

### Device Interaction Rules (Important Note)

- **Virtual Emulators**: Run headless (`-no-window -no-audio -no-boot-anim -gpu host`). Use `scrcpy` to display their screen and control them with mouse and keyboard (`--mouse=sdk --keyboard=sdk`).
- **Physical USB Devices**: Used by hand directly. Screen mirroring via `scrcpy` is **not** required and should not be offered or triggered for physical devices.

### Module Structure

```
src/
├── main.rs            # Entrypoint, terminal initialization, async event loop
├── model.rs           # Core domain models (AvdInfo, DeviceTarget, TargetType, etc.)
├── events.rs          # Channel event types (Key, Tick, DevicesRefreshed, ActionLog)
├── app.rs             # Application state, navigation, active pane focus
├── ui/
│   ├── mod.rs         # Root layout (header, tabs, status bar)
│   ├── emulator.rs    # Emulator & device management view (dual pane + drawer)
│   ├── build.rs       # Gradle build view
│   └── logs.rs        # Logcat stream view
└── android/
    ├── mod.rs         # Android subsystem coordinator
    ├── discovery.rs   # Virtual device discovery & ADB device parsing
    ├── lifecycle.rs   # Headless emulator start, boot status polling, scrcpy, kill
    └── deploy.rs      # Gradle build executor & ADB deployment
```

---

## 4. Android Tooling Integration Reference

### A. Device Discovery

- **Installed AVDs**:

    ```bash
    emulator -list-avds
    ```

    Returns newline-delimited list of AVD names (e.g. `Pixel_6a`, `medium_phone`).

- **Active Devices (USB & Running Emulators)**:
    ```bash
    adb devices -l
    ```
    Returns attached devices with format:
    `<serial> <state> [usb:<bus-port>] [product:<name>] [model:<name>] [device:<name>] [transport_id:<id>]`
    _Emulators_ typically have serial `emulator-<port>` (e.g., `emulator-5554`) or product/device with sdk tags.
    _Physical USB devices_ include `usb:<port>` in metadata.

### B. Virtual Device Lifecycle

- **Launch Headless (with Host GPU acceleration)**:

    ```bash
    emulator -avd <AVD_NAME> -no-window -no-audio -no-boot-anim -gpu host
    ```

    Spawn detached as a background process.

- **Check Boot Completion**:

    ```bash
    adb -s <SERIAL> shell getprop sys.boot_completed
    ```

    Returns `1` when boot is finished. Poll periodically after launching until `1` before dispatching APK installs.

- **Stop / Kill Emulator**:
    ```bash
    adb -s <SERIAL> emu kill
    ```
    Fallback if unresponsive: signal termination (`SIGTERM`/`SIGKILL`) on PID.

### C. Scrcpy Display Mirroring (Virtual Devices Only)

- **Launch Display Client for Emulator**:
    ```bash
    scrcpy -s <SERIAL> --window-title "<TITLE>" --stay-awake --mouse=sdk --keyboard=sdk
    ```
    Spawn detached in background. Note: ONLY applicable for emulator devices. Physical phones are operated directly by hand.

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
- **Launch App via Activity Manager**:
    ```bash
    adb -s <SERIAL> shell am start -n <PACKAGE>/<ACTIVITY> -a android.intent.action.MAIN -c android.intent.category.LAUNCHER
    ```
- **Stop App**:
    ```bash
    adb -s <SERIAL> shell am force-stop <PACKAGE>
    ```

### E. Logcat & PID Resolution

- **Resolve Package PID**:
    ```bash
    adb -s <SERIAL> shell pidof <PACKAGE>
    ```
- **Stream Logs**:
    ```bash
    adb -s <SERIAL> logcat -v time --pid=<PID>
    ```
    Stream continuously via asynchronous stdout piping into a ring buffer or `tui-term`.

---

## 5. Development & Testing Instructions

- **Build**: `cargo build`
- **Run**: `cargo run`
- **Check**: `cargo check` / `cargo clippy`
- **Prerequisites**:
    - `adb` and `emulator` must be in PATH or `$ANDROID_HOME` / `$ANDROID_SDK_ROOT`.
    - `scrcpy` optional but recommended for visual device display.
