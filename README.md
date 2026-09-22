# OpenWire EN ReadMe

### [RU ReadMe](README.ru.md)

> A native Linux soundpad and real-time audio effects processor for PipeWire.

OpenWire helps you trigger sound effects quickly, process microphone audio, and
send the processed signal to a virtual microphone. It is designed for
streaming, voice chat, gaming, and any workflow where low latency and keyboard
control matter.

> **Design note:** the visual concept and part of the interface design were
> created with the help of AI and then adapted to fit the project.

## Contents

- [Features](#features)
- [How It Works](#how-it-works)
- [Requirements](#requirements)
- [Running from Source](#running-from-source)
- [Building an Arch Linux Package](#building-an-arch-linux-package)
- [Using the Soundboard](#using-the-soundboard)
- [Hotkeys](#hotkeys)
- [Project Structure](#project-structure)
- [Development](#development)
- [Troubleshooting](#troubleshooting)

## Features

### Soundboard

- dynamically add pads without a fixed limit;
- play MP3, WAV, FLAC, OGG, AAC, and M4A files;
- `one-shot`, `loop`, and `toggle` playback modes;
- separate monitor and virtual-microphone volume controls;
- waveform preview and trim start/end controls;
- bass boost and a global stop command.

### Microphone Processing

- pitch and formant shifting;
- four-band equalizer;
- radio effects: band-pass, saturation, and noise bed;
- soft noise suppression with an adjustable threshold;
- microphone ducking while pads are playing;
- an **Ordinary Voice** preset for returning to neutral settings;
- a separate **Hear Yourself** monitor switch.

### Application Control

- global hotkeys;
- system tray support;
- open, hide, and close the window from the tray menu;
- PipeWire virtual microphone;
- dark interface;
- low-latency audio graph.

## How It Works

```text
Physical microphone
        │
        ▼
  PipeWire capture
        │
        ▼
    DSP chain ───────────────► OpenWire virtual microphone
        │
        └────────────────────► Local monitoring

Pads ────────────────────────► Virtual microphone + monitoring
```

The frontend provides the interface and controls. The Rust backend handles
PipeWire, DSP, audio decoding, global hotkeys, presets, and persistence.

## Requirements

- Linux;
- PipeWire and access to audio devices;
- Rust 1.77 or newer;
- Cargo;
- Node.js and npm;
- Tauri/WebKit system dependencies.

Global hotkey support depends on the desktop session. Hotkeys usually work
globally on X11. On Wayland, keyboard capture is controlled by the compositor
and its security policy.

## Running from Source

```bash
git clone <REPOSITORY-URL>
cd OpenWire
npm install
npm run build
cargo run --manifest-path src-tauri/Cargo.toml
```

`npm run build` compiles the CSS and copies JavaScript and fonts into `ui/`.
Tauri uses `ui/` as its frontend directory.

For Arch Linux, build dependencies can be installed with:

```bash
sudo pacman -S --needed base-devel rust nodejs npm pipewire webkit2gtk-4.1
```

Package names may differ on other distributions.

## Building an Arch Linux Package

```bash
cd packaging
./build.sh
```

The resulting `openwire-*.pkg.tar.zst` file will be placed in `packaging/`.
Built packages are best published in GitHub **Releases** rather than committed
to the source repository.

## Using the Soundboard

### Adding a Sound

1. Launch OpenWire.
2. Select a bank.
3. Click **Add pad** or select an empty pad.
4. Import an audio file.
5. Configure the volume, playback mode, trim range, and hotkey.

### Configuring the Voice Chain

1. Open the DSP section.
2. Select **Ordinary Voice** to return to neutral processing.
3. Enable noise suppression and adjust its threshold if needed.
4. Disable **Hear Yourself** if you do not want microphone monitoring.
5. Save a suitable configuration as a preset.

DSP presets use TOML format. An example is available in
`examples/presets/`.

## Hotkeys

Supported keys include:

- letters and numbers;
- F1–F12;
- NumPad;
- Space, Tab, Enter, and Backspace;
- Insert, Delete, Home, and End;
- Page Up and Page Down;
- arrow keys, CapsLock, PrintScreen, ScrollLock, and Pause.

A hotkey can be assigned in the properties panel of the selected pad. The
global listener continues working while the window is hidden in the system tray.

## Project Structure

```text
src/
├── css/             source styles and design tokens
├── js/              frontend source code
└── fonts/           bundled fonts

src-tauri/
├── src/             Rust code and PipeWire audio engine
├── capabilities/   Tauri permissions
├── icons/           application icons
└── tauri.conf.json  Tauri configuration

ui/                  generated Tauri frontend
examples/            preset examples
packaging/           PKGBUILD and package build script
scripts/             build helper scripts
tests/               frontend smoke tests
```

The following files are generated automatically and should not be edited by
hand:

```text
ui/app.css
ui/app-icon.svg
ui/fonts/
ui/js/
```

Local user data should not be published:

```text
settings.toml
soundboard.toml
Voices/
```

## Development

Run the frontend tests:

```bash
npm test
```

Check and test the Rust backend:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Run all checks before submitting changes.

## Troubleshooting

### No Audio

Make sure PipeWire is running and audio devices are available:

```bash
systemctl --user status pipewire pipewire-pulse
pw-cli info all
```

### Choppy Audio

- check CPU usage;
- make sure PipeWire is not using an excessively large quantum;
- temporarily disable heavy DSP effects;
- check for conflicting audio routing or virtual devices.

### Global Hotkeys Do Not Work

On X11, verify that the application can access input devices. On Wayland,
check the compositor restrictions: passive global keyboard capture may be
blocked by the desktop session.

### The Window Disappeared

This is expected behavior: closing the window hides the application in the
system tray. Open the OpenWire tray menu and select **Open window**.

## Feedback

When reporting a problem, include:

- your distribution and desktop session;
- the PipeWire version;
- how OpenWire was launched;
- steps to reproduce the issue;
- terminal output or error logs.
