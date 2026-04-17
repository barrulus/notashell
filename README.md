# notashell

A lightweight, native system control panel for Wayland compositors. Built with Rust, GTK4, and layer-shell — manages WiFi, Bluetooth, volume, brightness, and night mode as a proper alternative to `nmtui`, `nm-applet`, `blueman`, and rofi-based scripts.

This is a direct fork of the work done in https://github.com/Vijay-papanaboina/wifi-manager

> **Status:** notashell is under active development. Interfaces and configuration may change between releases.


## Table of Contents

- [Why](#why)
- [Features](#features)
- [Installation](#installation)
  - [Build from Source](#build-from-source)
- [Usage](#usage)
  - [Hyprland Integration](#hyprland-integration)
  - [Niri Integration](#niri-integration)
- [Configuration](#configuration)
- [Theming](#theming)
- [Architecture](#architecture)
- [Tech Stack](#tech-stack)
- [License](#license)

## Why

There is no widely adopted standalone GUI WiFi manager designed specifically for Wayland compositors:

| Existing tool          | Problem                                                          |
| ---------------------- | ---------------------------------------------------------------- |
| `nm-applet`            | Tray-based, scan/connect dropdown broken on Wayland              |
| `nm-connection-editor` | Only edits saved connections, no scanning                        |
| `nmtui`                | Terminal TUI, not a GUI                                          |
| `iwgtk`                | Requires iwd, most distros use NetworkManager                    |
| Rofi/wofi scripts      | No real UI — no signal bars, no live updates, no visual feedback |

**notashell** fills this gap: a floating panel that manages WiFi and Bluetooth with a proper GUI, live state updates, and full theming support.

## Features

### WiFi

- **Scan and list** available WiFi networks with signal strength, frequency band, and security info
- **Connect** to open, WPA2, and WPA3 networks with inline password entry
- **Saved network detection** — reconnects to known networks without re-entering passwords
- **Live updates** — UI reflects WiFi state changes in real time (D-Bus signal subscriptions)
- **Scan-on-show** — automatically rescans when the panel is toggled visible
- **WiFi toggle** — enable/disable the wireless radio directly from the panel
- **Forget network** — remove saved connections via the ⋮ menu on each network

### Bluetooth

- **Device discovery** — scan for nearby Bluetooth devices
- **Connect/disconnect** — manage paired and new devices
- **Pairing** — "Just Works" pairing with auto-trust for new devices
- **Power toggle** — enable/disable the Bluetooth adapter
- **Live updates** — device list refreshes automatically via BlueZ D-Bus signals
- **Device categories** — icons for audio, phone, computer, input, and other device types
- **Remove device** — unpair devices via the ⋮ menu
- **Graceful fallback** — BT tab is hidden if no Bluetooth adapter is detected

### General

- **Brightness & Volume Controls** — dedicated sliders statically pinned to the bottom of the panel,
  syncing in real-time with system events via `libpulse` and `systemd-logind`
- **Night Mode (Color Temperature)** — dedicated slider to adjust display warmth,
  powered by Wayland's `wlr-gamma-control` protocol
- **Tabbed interface** — switch between WiFi and Bluetooth tabs
- **Context-aware toggle** — single switch controls WiFi or Bluetooth power based on active tab
- **Daemon mode** — runs as a background process, toggled via CLI flag or D-Bus
- **Layer-shell overlay** — floating panel with no window decorations, positioned via config
- **Configurable position** — 9 anchor positions with per-edge margin offsets
- **Theming** — override the default dark theme with CSS rules in your KDL config
- **Customizable signal icons** — configure signal strength icons via config
- **Forget network** — remove saved connections via the ⋮ menu on each network
- **Live reload** — reload config and theme without restarting (`--reload`)
- **Escape to close** — press Escape to hide the panel

## Installation

### Runtime Dependencies

The following must be installed and running on your system:

- **NetworkManager** — system network service
- **BlueZ** — Bluetooth protocol stack (optional — BT tab is hidden if unavailable)
- **PulseAudio / PipeWire-Pulse** — Audio server for volume control integration
- **systemd-logind** — Session manager for brightness control (standard on systemd distros)
- **GTK4** — UI toolkit
- **gtk4-layer-shell** — Wayland layer-shell integration

### Build from Source

**Requirements:**

- Linux with Wayland (Hyprland, Sway, or any wlroots-based compositor)
- [NetworkManager](https://networkmanager.dev/) as the system network service
- [BlueZ](http://www.bluez.org/) for Bluetooth support (optional)
- GTK4 and gtk4-layer-shell libraries
- Rust toolchain (1.85+)

**System Dependencies:**


**Build:**

```sh
cargo build --release
sudo install -Dm755 target/release/notashell /usr/local/bin/notashell
```

## Usage

```sh
# Launch the daemon (panel starts hidden, then shown on first load)
notashell

# Toggle panel visibility
notashell --toggle

# Reload config and CSS without restarting
notashell --reload

# Toggle between compact and expanded panel height
# (layer-shell panels have no grab handle, so size is toggled explicitly)
notashell --resize
```

Inside the panel, <kbd>Ctrl</kbd>+<kbd>E</kbd> toggles the expanded height (same
action as `--resize`) and <kbd>Esc</kbd> hides the panel.

### Hyprland Integration

Add to your Hyprland config:

```ini
# Autostart and keybind
exec-once = notashell
bind = $mainMod, W, exec, notashell --toggle

# Optional: blur and styling for the panel
layerrule = blur on, match:namespace notashell
layerrule = ignore_alpha 0.3, match:namespace notashell
```

The layer namespace is `notashell` (visible in `hyprctl layers`). You can target it with any Hyprland `layerrule` — blur, shadows, animations, etc.

### Niri Integration

Add to your `~/.config/niri/config.kdl`:

```kdl
// Autostart
spawn-at-startup "notashell"

// Keybinding to toggle
binds {
    Mod+W { spawn "notashell" "--toggle"; }
}

// Optional: styling for the panel
layer-rule {
    match namespace="notashell"
    opacity 0.95
    shadow {
        on
    }
}
```

The layer namespace is `notashell`. You can target it with any niri `layer-rule` property — opacity, shadow, corner radius, etc.


## Configuration

Configuration is loaded from `~/.config/notashell/config.kdl`. On first run, a default config file is created with all options commented out so you can see what's available. All fields are optional and fall back to defaults.

```kdl
// Window position on screen
// Options: center, top-right, top-center, top-left,
//          bottom-right, bottom-center, bottom-left,
//          center-right, center-left
position "center"

// Margin offsets in pixels (only effective on anchored edges)
margin {
    top 10
    right 10
    bottom 10
    left 10
}

// Custom icons (Nerd Fonts)
icons {
    // Signal strength: weak, fair, good, strong
    signal "󰤟" "󰤢" "󰤥" "󰤨"
    lock ""
    saved ""
}

// Show the panel immediately when the daemon starts
show-on-start false
```

> **Note:** Margins only apply to edges the window is anchored to. For example, with `top-left`, only `margin { top ... }` and `margin { left ... }` have an effect. With `center`, no margins apply.

**Signal Icon Ranges:**

- Signal icon 1 (weak): 0-24% signal strength
- Signal icon 2 (fair): 25-49% signal strength
- Signal icon 3 (good): 50-74% signal strength
- Signal icon 4 (strong): 75-100% signal strength

### Includes

Config files can include other KDL files for composition:

```kdl
include "theme.kdl"
include optional=true "local-overrides.kdl"
```

Paths are relative to the containing file. `optional=true` silently skips missing files. Cyclic includes are detected and skipped.

## Theming

notashell ships with a dark default theme. Theme overrides are defined in the `theme` block of your config file. Rules generate CSS loaded after the default theme — only include rules you want to change.

```kdl
theme {
    rule ".notashell-panel" {
        background "rgba(20, 22, 30, 0.95)"
        border-radius "16px"
    }
    rule ".scan-button:hover" {
        background "rgba(255, 255, 255, 0.12)"
        color "#ffffff"
    }
}
```

Multiple `theme` blocks are supported (e.g. from included files) — their CSS is concatenated.

See `examples/style-reference.css` for all available CSS selectors and their default values.

**Available selectors:**

| Selector                 | Element                        |
| ------------------------ | ------------------------------ |
| `.notashell-panel`       | Main window container          |
| `.header`                | Top bar (toggle, status, scan) |
| `.tab-bar`               | Tab container                  |
| `.tab-button`            | Wi-Fi / Bluetooth tab button   |
| `.network-list`          | Scrollable network list        |
| `.network-row`           | Individual network entry       |
| `.network-row.connected` | Connected network              |
| `.network-row.saved`     | Known/saved network            |
| `.ssid-label`            | Network name                   |
| `.signal-icon`           | Signal strength indicator      |
| `.security-icon`         | Lock/open icon                 |
| `.device-list`           | Bluetooth device list          |
| `.device-row`            | Individual Bluetooth device    |
| `.device-row.connected`  | Connected Bluetooth device     |
| `.device-name`           | Bluetooth device name          |
| `.device-icon`           | Device category icon           |
| `.trusted-icon`          | Trusted device indicator       |
| `.password-entry`        | Password input field           |
| `.connect-button`        | Connect action button          |
| `.error-label`           | Error messages                 |

## Architecture

```
src/
├── main.rs                  # Entry point, CLI parsing, GTK application setup
├── config.rs                # Configuration loader (KDL)
├── daemon.rs                # D-Bus daemon service (Toggle/Show/Hide)
├── app/
│   ├── mod.rs               # App state and setup (WiFi + Bluetooth)
│   ├── scanning.rs          # WiFi scan logic and polling
│   ├── connection.rs        # WiFi toggle, network click, password actions
│   ├── live_updates.rs      # WiFi D-Bus signal subscriptions
│   ├── bluetooth.rs         # Bluetooth controller (scan, connect, power)
│   ├── bt_live_updates.rs   # Bluetooth D-Bus signal subscriptions
│   ├── controls.rs          # Wires GTK controls UI to backend managers
│   └── shortcuts.rs         # Keyboard shortcuts and hot-reload
├── controls/
│   ├── mod.rs               # Entry point for backend controls
│   ├── brightness.rs        # BrightnessManager (systemd-logind + sysfs)
│   ├── volume.rs            # VolumeManager (libpulse-binding)
│   └── night_mode.rs        # NightModeManager (Wayland wlr-gamma-control)
├── dbus/
│   ├── proxies.rs           # NetworkManager D-Bus proxy traits (zbus)
│   ├── network_manager.rs   # High-level WiFi operations
│   ├── access_point.rs      # WiFi data model (Network, SecurityType, Band)
│   ├── connection.rs        # NM connection settings builders
│   ├── bluez_proxies.rs     # BlueZ D-Bus proxy traits (Adapter1, Device1)
│   ├── bluetooth_manager.rs # High-level Bluetooth operations
│   └── bluetooth_device.rs  # Bluetooth data model (BluetoothDevice, DeviceCategory)
└── ui/
    ├── window.rs            # Layer-shell window setup, tab stack
    ├── header.rs            # Header bar with tab switcher
    ├── controls_panel.rs    # Brightness and Volume sliders (footer)
    ├── network_list.rs      # WiFi network list
    ├── network_row.rs       # WiFi network row widget
    ├── device_list.rs       # Bluetooth device list
    ├── device_row.rs        # Bluetooth device row widget
    └── password_dialog.rs   # Inline password entry
```

## Tech Stack

| Component           | Library                                 |
| ------------------- | --------------------------------------- |
| Language            | Rust                                    |
| UI framework        | GTK4                                    |
| Wayland integration | gtk4-layer-shell / wayland-client       |
| D-Bus client        | zbus (pure Rust, async-io backend)      |
| WiFi backend        | NetworkManager (D-Bus)                  |
| Bluetooth backend   | BlueZ (D-Bus)                           |
| Audio backend       | libpulse (PulseAudio or PipeWire-Pulse) |
| Brightness backend  | systemd-logind (D-Bus via zbus)         |
| Night Mode backend  | wlr-gamma-control (Wayland)             |
| Configuration       | kdl                                     |
| CLI                 | clap                                    |

## License

MIT
