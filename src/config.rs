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

// Theme overrides — generates CSS loaded after the default theme.
// Only include rules you want to change; everything else uses defaults.
// See examples/style-reference.css for all available selectors.
//
// theme {
//     rule ".notashell-panel" {
//         background "rgba(20, 22, 30, 0.95)"
//         border-radius "16px"
//     }
//     rule ".scan-button:hover" {
//         background "rgba(255, 255, 255, 0.12)"
//         color "#ffffff"
//     }
//     rule ".network-row:hover" {
//         background "rgba(180, 190, 254, 0.08)"
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
