//! Application configuration loaded from `~/.config/notashell/config.kdl`.
//!
//! Supports an `include` directive for file composition and a `theme` block
//! that generates GTK CSS at runtime.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Default config file content written on first run.
/// All options are commented out so the app uses its built-in defaults,
/// but users can see and uncomment what they want to change.
const DEFAULT_CONFIG: &str = r##"// notashell configuration
// Uncomment and edit any option below to override the default.

// Include additional config files (paths relative to this file)
// include "theme.kdl"
// include optional=true "local-overrides.kdl"

// Window position on screen
// Options: center, top-right, top-center, top-left,
//          bottom-right, bottom-center, bottom-left,
//          center-right, center-left
// position "center"

// Margin offsets in pixels (only effective on anchored edges)
// margin {
//     top 10
//     right 10
//     bottom 10
//     left 10
// }

// Custom icons (Nerd Fonts)
// icons {
//     // Signal strength: weak, fair, good, strong
//     signal "󰤟" "󰤢" "󰤥" "󰤨"
//     // Alternative ASCII icons:
//     // signal "▂___" "▂▄__" "▂▄▆_" "▂▄▆█"
//     lock ""
//     saved ""
// }

// Show the panel immediately when the daemon starts
// show-on-start false

// ── Theme Overrides ────────────────────────────────────────────────────────
// Generates CSS loaded after the default theme. Uncomment any rule to
// override it. Only uncommented rules take effect; everything else uses
// the built-in defaults.
//
// theme {
//
//     // ── Panel ────────────────────────────────────────────────────────
//     rule ".notashell-panel" {
//         background "rgba(20, 22, 30, 0.95)"
//         border-radius "16px"
//         border "1px solid rgba(255, 255, 255, 0.08)"
//         box-shadow "0 16px 48px rgba(0, 0, 0, 0.6)"
//     }
//     rule ".header-separator" {
//         background-color "rgba(255, 255, 255, 0.06)"
//     }
//
//     // ── Header ───────────────────────────────────────────────────────
//     rule ".header" {
//         padding "16px 20px"
//     }
//     rule ".header-title" {
//         color "#ffffff"
//         font-size "16px"
//         font-weight "700"
//     }
//     rule ".status-label" {
//         color "rgba(255, 255, 255, 0.45)"
//         font-size "11px"
//     }
//     rule ".scan-button" {
//         min-width "32px"
//         min-height "32px"
//         border-radius "50%"
//         background "rgba(255, 255, 255, 0.06)"
//         color "rgba(255, 255, 255, 0.6)"
//     }
//     rule ".scan-button:hover" {
//         background "rgba(255, 255, 255, 0.12)"
//         color "#ffffff"
//     }
//
//     // ── Switch ───────────────────────────────────────────────────────
//     rule "switch" {
//         min-width "44px"
//         min-height "22px"
//         border-radius "999px"
//         background "rgba(255, 255, 255, 0.12)"
//     }
//     rule "switch:checked" {
//         background "#3584e4"
//     }
//     rule "switch slider" {
//         min-width "18px"
//         min-height "18px"
//         border-radius "999px"
//         background "#ffffff"
//     }
//
//     // ── Tab Bar ──────────────────────────────────────────────────────
//     rule ".tab-bar" {
//         padding "14px 20px 6px"
//     }
//     rule ".tab-button" {
//         background "transparent"
//         color "rgba(255, 255, 255, 0.5)"
//         border-radius "10px"
//         font-size "13px"
//         font-weight "600"
//     }
//     rule ".tab-button:hover" {
//         color "rgba(255, 255, 255, 0.7)"
//         background "rgba(255, 255, 255, 0.04)"
//     }
//     rule ".tab-button:checked" {
//         background "rgba(53, 132, 228, 0.15)"
//         color "#78aeed"
//     }
//
//     // ── Wi-Fi Network List ──────────────────────────────────────────
//     rule ".network-list" {
//         background "transparent"
//     }
//     rule ".network-scroll scrollbar slider" {
//         background "rgba(255, 255, 255, 0.08)"
//         border-radius "99px"
//         min-width "4px"
//     }
//     rule ".network-row" {
//         padding "12px 20px"
//         margin "2px 8px"
//         border-radius "12px"
//     }
//     rule ".network-row:hover" {
//         background "rgba(255, 255, 255, 0.05)"
//     }
//     rule ".network-row.connected" {
//         background "rgba(53, 132, 228, 0.12)"
//     }
//     rule ".network-row.connected .ssid-label" {
//         color "#ffffff"
//         font-weight "700"
//     }
//     rule ".ssid-label" {
//         color "rgba(255, 255, 255, 0.9)"
//         font-size "14px"
//         font-weight "500"
//     }
//     rule ".network-subtitle" {
//         color "rgba(255, 255, 255, 0.4)"
//         font-size "11px"
//     }
//     rule ".network-row.connected .network-subtitle" {
//         color "rgba(255, 255, 255, 0.7)"
//     }
//     rule ".signal-icon" {
//         font-size "16px"
//         min-width "24px"
//     }
//     rule ".signal-strong" {
//         color "#57e389"
//     }
//     rule ".signal-good" {
//         color "#f9f06b"
//     }
//     rule ".signal-fair" {
//         color "#ffbe6f"
//     }
//     rule ".signal-weak" {
//         color "#f5c211"
//     }
//     rule ".security-icon" {
//         font-size "14px"
//         color "rgba(255, 255, 255, 0.4)"
//     }
//     rule ".saved-icon" {
//         font-size "14px"
//         color "#3584e4"
//     }
//     rule ".network-menu-btn" {
//         color "rgba(255, 255, 255, 0.3)"
//         border-radius "50%"
//     }
//     rule ".network-menu-btn:hover" {
//         background "rgba(255, 255, 255, 0.1)"
//         color "#ffffff"
//     }
//     rule ".network-popover" {
//         background "#242424"
//         border "1px solid rgba(255, 255, 255, 0.1)"
//         border-radius "12px"
//     }
//     rule ".network-popover modelbutton" {
//         color "#ff7b63"
//     }
//     rule ".network-popover modelbutton:hover" {
//         background "rgba(255, 123, 99, 0.1)"
//     }
//
//     // ── Password Dialog ─────────────────────────────────────────────
//     rule ".password-section" {
//         padding "16px 20px"
//         background "rgba(255, 255, 255, 0.03)"
//         border-top "1px solid rgba(255, 255, 255, 0.08)"
//     }
//     rule ".password-title" {
//         color "#ffffff"
//         font-size "12px"
//         font-weight "600"
//     }
//     rule ".password-entry" {
//         background "rgba(0, 0, 0, 0.2)"
//         border "1px solid rgba(255, 255, 255, 0.1)"
//         border-radius "10px"
//         color "#ffffff"
//     }
//     rule ".password-entry:focus" {
//         border-color "#3584e4"
//         box-shadow "0 0 0 2px rgba(53, 132, 228, 0.3)"
//     }
//     rule ".connect-button" {
//         background "#3584e4"
//         color "#ffffff"
//         border-radius "10px"
//         font-weight "700"
//     }
//     rule ".connect-button:hover" {
//         background "#4a91e9"
//     }
//     rule ".cancel-button" {
//         background "transparent"
//         color "rgba(255, 255, 255, 0.6)"
//         border-radius "10px"
//     }
//     rule ".cancel-button:hover" {
//         background "rgba(255, 255, 255, 0.08)"
//     }
//
//     // ── Bluetooth Device List ────────────────────────────────────────
//     rule ".device-list" {
//         background "transparent"
//     }
//     rule ".device-scroll scrollbar slider" {
//         background "rgba(255, 255, 255, 0.08)"
//         border-radius "99px"
//         min-width "4px"
//     }
//     rule ".device-row" {
//         padding "12px 20px"
//         margin "2px 8px"
//         border-radius "12px"
//     }
//     rule ".device-row:hover" {
//         background "rgba(255, 255, 255, 0.05)"
//     }
//     rule ".device-row.connected" {
//         background "rgba(53, 132, 228, 0.12)"
//     }
//     rule ".device-row.connected .device-name" {
//         color "#ffffff"
//         font-weight "700"
//     }
//     rule ".device-name" {
//         color "rgba(255, 255, 255, 0.9)"
//         font-size "14px"
//         font-weight "500"
//     }
//     rule ".device-subtitle" {
//         color "rgba(255, 255, 255, 0.4)"
//         font-size "11px"
//     }
//     rule ".device-row.connected .device-subtitle" {
//         color "rgba(255, 255, 255, 0.7)"
//     }
//     rule ".device-icon" {
//         font-size "16px"
//         min-width "24px"
//         color "rgba(255, 255, 255, 0.6)"
//     }
//     rule ".device-row.connected .device-icon" {
//         color "#3584e4"
//     }
//     rule ".trusted-icon" {
//         font-size "14px"
//         color "#57e389"
//     }
//     rule ".device-menu-btn" {
//         color "rgba(255, 255, 255, 0.3)"
//         border-radius "50%"
//     }
//     rule ".device-menu-btn:hover" {
//         background "rgba(255, 255, 255, 0.1)"
//         color "#ffffff"
//     }
//     rule ".device-popover" {
//         background "#242424"
//         border "1px solid rgba(255, 255, 255, 0.1)"
//         border-radius "12px"
//     }
//     rule ".device-popover modelbutton" {
//         color "#ff7b63"
//     }
//     rule ".device-popover modelbutton:hover" {
//         background "rgba(255, 123, 99, 0.1)"
//     }
//
//     // ── Audio Mixer ─────────────────────────────────────────────────
//     rule ".mixer-container" {
//         background "transparent"
//     }
//     rule ".mixer-section-header" {
//         color "rgba(255, 255, 255, 0.45)"
//         font-size "11px"
//         font-weight "700"
//         text-transform "uppercase"
//     }
//     rule ".mixer-list" {
//         background "transparent"
//     }
//     rule ".mixer-scroll scrollbar slider" {
//         background "rgba(255, 255, 255, 0.08)"
//         border-radius "99px"
//         min-width "4px"
//     }
//     rule ".mixer-row" {
//         padding "8px 12px"
//         margin "2px 8px"
//         border-radius "12px"
//     }
//     rule ".mixer-row:hover" {
//         background "rgba(255, 255, 255, 0.05)"
//     }
//     rule ".mixer-row.default" {
//         background "rgba(53, 132, 228, 0.12)"
//     }
//     rule ".mixer-row.default .mixer-name" {
//         color "#ffffff"
//         font-weight "700"
//     }
//     rule ".mixer-icon" {
//         font-size "14px"
//         min-width "20px"
//         color "rgba(255, 255, 255, 0.6)"
//     }
//     rule ".mixer-row.default .mixer-icon" {
//         color "#3584e4"
//     }
//     rule ".mixer-name" {
//         color "rgba(255, 255, 255, 0.9)"
//         font-size "12px"
//         font-weight "500"
//     }
//     rule ".mixer-scale trough" {
//         background "rgba(255, 255, 255, 0.08)"
//         border-radius "99px"
//         min-height "4px"
//     }
//     rule ".mixer-scale trough highlight" {
//         background "#3584e4"
//         border-radius "99px"
//     }
//     rule ".mixer-scale.muted trough highlight" {
//         background "rgba(255, 255, 255, 0.15)"
//     }
//     rule ".mixer-scale slider" {
//         background "#ffffff"
//         border-radius "50%"
//         min-width "14px"
//         min-height "14px"
//     }
//     rule ".mixer-mute-btn" {
//         min-width "28px"
//         min-height "28px"
//         border-radius "50%"
//         color "rgba(255, 255, 255, 0.5)"
//     }
//     rule ".mixer-mute-btn:hover" {
//         background "rgba(255, 255, 255, 0.1)"
//         color "#ffffff"
//     }
//
//     // ── Controls Panel ──────────────────────────────────────────────
//     rule ".controls-panel" {
//         border-top "1px solid rgba(255, 255, 255, 0.06)"
//     }
//     rule ".controls-panel scale trough" {
//         background "rgba(255, 255, 255, 0.08)"
//         border-radius "99px"
//         min-height "6px"
//     }
//     rule ".controls-panel scale trough highlight" {
//         background "#3584e4"
//         border-radius "99px"
//     }
//     rule ".controls-panel scale slider" {
//         background "#ffffff"
//         border-radius "50%"
//         min-width "16px"
//         min-height "16px"
//     }
//     rule ".controls-panel scale value" {
//         color "rgba(255, 255, 255, 0.5)"
//         font-size "11px"
//     }
//
//     // ── Tooltips ────────────────────────────────────────────────────
//     rule "tooltip" {
//         background "rgba(3, 17, 45, 0.5)"
//         color "#ffffff"
//         border-radius "8px"
//     }
//
//     // ── Dialog Buttons ──────────────────────────────────────────────
//     rule "window.dialog button" {
//         background "rgba(255, 255, 255, 0.08)"
//         color "rgba(255, 255, 255, 0.9)"
//         border-radius "10px"
//         font-weight "600"
//     }
//     rule "window.dialog button:hover" {
//         background "rgba(255, 255, 255, 0.15)"
//         color "#ffffff"
//     }
//     rule "window.dialog button:last-child" {
//         background "rgba(224, 27, 36, 0.8)"
//         color "#ffffff"
//     }
//     rule "window.dialog button:last-child:hover" {
//         background "#e01b24"
//     }
// }
"##;

/// Window position on screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Center,
    TopRight,
    TopCenter,
    TopLeft,
    BottomRight,
    BottomCenter,
    BottomLeft,
    CenterRight,
    CenterLeft,
}

impl Position {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "center" => Some(Self::Center),
            "top-right" => Some(Self::TopRight),
            "top-center" => Some(Self::TopCenter),
            "top-left" => Some(Self::TopLeft),
            "bottom-right" => Some(Self::BottomRight),
            "bottom-center" => Some(Self::BottomCenter),
            "bottom-left" => Some(Self::BottomLeft),
            "center-right" => Some(Self::CenterRight),
            "center-left" => Some(Self::CenterLeft),
            _ => None,
        }
    }
}

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Window position (default: "center")
    pub position: Position,

    /// Margin from top edge in pixels
    pub margin_top: i32,

    /// Margin from right edge in pixels
    pub margin_right: i32,

    /// Margin from bottom edge in pixels
    pub margin_bottom: i32,

    /// Margin from left edge in pixels
    pub margin_left: i32,

    /// Custom signal strength icons [weak, fair, good, strong]
    pub signal_icons: [String; 4],

    /// Custom lock icon for secured networks
    pub lock_icon: String,

    /// Custom saved icon for saved networks
    pub saved_icon: String,

    /// Whether to show the panel when the daemon starts (default: false)
    pub show_on_start: bool,

    /// Generated CSS from theme blocks
    theme_css: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            position: Position::default(),
            margin_top: 10,
            margin_right: 10,
            margin_bottom: 10,
            margin_left: 10,
            signal_icons: [
                "󰤟".to_string(), // weak
                "󰤢".to_string(), // fair
                "󰤥".to_string(), // good
                "󰤨".to_string(), // strong
            ],
            lock_icon: "".to_string(),
            saved_icon: "".to_string(),
            show_on_start: false,
            theme_css: String::new(),
        }
    }
}

impl Config {
    /// Load config from `~/.config/notashell/config.kdl`.
    /// Falls back to defaults if file doesn't exist or has errors.
    pub fn load() -> Self {
        let Some(path) = config_file_path() else {
            return Self::default();
        };

        if !path.exists() {
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    log::warn!("Failed to create config directory: {e}");
                } else if let Err(e) = std::fs::write(&path, DEFAULT_CONFIG) {
                    log::warn!("Failed to write default config: {e}");
                } else {
                    log::info!("Created default config at {path:?}");
                }
            }
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => match contents.parse::<kdl::KdlDocument>() {
                Ok(doc) => {
                    let mut config = Self::default();
                    let mut visited = HashSet::new();
                    let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                    visited.insert(canonical);
                    let parent = path.parent().unwrap_or(Path::new("."));
                    config.apply_document(&doc, parent, &mut visited);
                    log::info!("Config loaded from {:?}", path);
                    config
                }
                Err(e) => {
                    log::warn!("Failed to parse config file: {e}, using defaults");
                    Self::default()
                }
            },
            Err(e) => {
                log::warn!("Failed to read config file: {e}, using defaults");
                Self::default()
            }
        }
    }

    /// Get the generated theme CSS.
    pub fn theme_css(&self) -> &str {
        &self.theme_css
    }

    /// Apply all nodes from a KDL document to this config.
    fn apply_document(
        &mut self,
        doc: &kdl::KdlDocument,
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
    ) {
        for node in doc.nodes() {
            match node.name().value() {
                "include" => self.handle_include(node, base_dir, visited),
                "position" => {
                    if let Some(s) = first_string_arg(node) {
                        match Position::from_str(s) {
                            Some(pos) => self.position = pos,
                            None => log::warn!("Unknown position: {s:?}"),
                        }
                    }
                }
                "margin" => {
                    if let Some(children) = node.children() {
                        for child in children.nodes() {
                            match child.name().value() {
                                "top" => {
                                    if let Some(v) = first_i64_arg(child) {
                                        self.margin_top = v as i32;
                                    }
                                }
                                "right" => {
                                    if let Some(v) = first_i64_arg(child) {
                                        self.margin_right = v as i32;
                                    }
                                }
                                "bottom" => {
                                    if let Some(v) = first_i64_arg(child) {
                                        self.margin_bottom = v as i32;
                                    }
                                }
                                "left" => {
                                    if let Some(v) = first_i64_arg(child) {
                                        self.margin_left = v as i32;
                                    }
                                }
                                other => log::warn!("Unknown margin field: {other:?}"),
                            }
                        }
                    }
                }
                "icons" => {
                    if let Some(children) = node.children() {
                        for child in children.nodes() {
                            match child.name().value() {
                                "signal" => {
                                    let args: Vec<&str> = child
                                        .entries()
                                        .iter()
                                        .filter(|e| e.name().is_none())
                                        .filter_map(|e| e.value().as_string())
                                        .collect();
                                    if args.len() == 4 {
                                        self.signal_icons = [
                                            args[0].to_string(),
                                            args[1].to_string(),
                                            args[2].to_string(),
                                            args[3].to_string(),
                                        ];
                                    } else {
                                        log::warn!(
                                            "signal expects 4 icons, got {}",
                                            args.len()
                                        );
                                    }
                                }
                                "lock" => {
                                    if let Some(s) = first_string_arg(child) {
                                        self.lock_icon = s.to_string();
                                    }
                                }
                                "saved" => {
                                    if let Some(s) = first_string_arg(child) {
                                        self.saved_icon = s.to_string();
                                    }
                                }
                                other => log::warn!("Unknown icon field: {other:?}"),
                            }
                        }
                    }
                }
                "show-on-start" => {
                    if let Some(b) = first_bool_arg(node) {
                        self.show_on_start = b;
                    }
                }
                "theme" => {
                    if let Some(children) = node.children() {
                        self.theme_css.push_str(&generate_theme_css(children));
                    }
                }
                other => log::warn!("Unknown config node: {other:?}"),
            }
        }
    }

    /// Process an `include` directive.
    fn handle_include(
        &mut self,
        node: &kdl::KdlNode,
        base_dir: &Path,
        visited: &mut HashSet<PathBuf>,
    ) {
        let optional = node
            .entries()
            .iter()
            .find(|e| e.name().map(|n| n.value()) == Some("optional"))
            .and_then(|e| e.value().as_bool())
            .unwrap_or(false);

        let Some(rel_path) = first_string_arg(node) else {
            log::warn!("include directive missing path argument");
            return;
        };

        let full_path = base_dir.join(rel_path);
        let canonical = match full_path.canonicalize() {
            Ok(p) => p,
            Err(_) if optional => {
                log::info!("Optional include not found: {rel_path:?}");
                return;
            }
            Err(e) => {
                log::warn!("Failed to resolve include path {rel_path:?}: {e}");
                return;
            }
        };

        if !visited.insert(canonical.clone()) {
            log::warn!("Skipping cyclic include: {rel_path:?}");
            return;
        }

        match std::fs::read_to_string(&canonical) {
            Ok(contents) => match contents.parse::<kdl::KdlDocument>() {
                Ok(doc) => {
                    let parent = canonical.parent().unwrap_or(base_dir);
                    self.apply_document(&doc, parent, visited);
                    log::info!("Included config from {:?}", canonical);
                }
                Err(e) => {
                    log::warn!("Failed to parse included file {rel_path:?}: {e}");
                }
            },
            Err(e) if optional => {
                log::info!("Optional include unreadable: {rel_path:?}: {e}");
            }
            Err(e) => {
                log::warn!("Failed to read included file {rel_path:?}: {e}");
            }
        }

        visited.remove(&canonical);
    }
}

/// Generate CSS from a `theme` block's children (each child is a `rule` node).
fn generate_theme_css(doc: &kdl::KdlDocument) -> String {
    let mut css = String::new();
    for node in doc.nodes() {
        if node.name().value() != "rule" {
            log::warn!("Unknown theme child: {:?}", node.name().value());
            continue;
        }
        let Some(selector) = first_string_arg(node) else {
            log::warn!("theme rule missing selector argument");
            continue;
        };
        let Some(children) = node.children() else {
            continue;
        };
        css.push_str(selector);
        css.push_str(" {\n");
        for prop in children.nodes() {
            let prop_name = prop.name().value();
            if let Some(val) = first_string_arg(prop) {
                css.push_str("  ");
                css.push_str(prop_name);
                css.push_str(": ");
                css.push_str(val);
                css.push_str(";\n");
            }
        }
        css.push_str("}\n");
    }
    css
}

/// Get the first positional string argument from a KDL node.
fn first_string_arg(node: &kdl::KdlNode) -> Option<&str> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .and_then(|e| e.value().as_string())
}

/// Get the first positional integer argument from a KDL node.
fn first_i64_arg(node: &kdl::KdlNode) -> Option<i64> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .and_then(|e| e.value().as_integer())
        .map(|v| v as i64)
}

/// Get the first positional boolean argument from a KDL node.
fn first_bool_arg(node: &kdl::KdlNode) -> Option<bool> {
    node.entries()
        .iter()
        .find(|e| e.name().is_none())
        .and_then(|e| e.value().as_bool())
}

/// Get the config file path: ~/.config/notashell/config.kdl
fn config_file_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("notashell")
            .join("config.kdl"),
    )
}
