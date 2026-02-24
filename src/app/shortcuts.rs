//! Shortcuts — keyboard and D-Bus triggered actions (Escape, reload, resize).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;

use crate::ui::window::{self, PanelWidgets};

use super::{AppState, refresh_list};

/// Set up Escape key handler to hide panel (with proper state tracking).
pub(super) fn setup_escape_key(widgets: &PanelWidgets, panel_state: crate::daemon::PanelState) {
    use gtk4::{gdk, glib, prelude::*, EventControllerKey};
    
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            panel_state.hide();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    widgets.window.add_controller(key_controller);
}

/// Poll the resize_requested flag and toggle expanded/compact scroll heights.
pub(super) fn setup_resize_on_request(
    widgets: &PanelWidgets,
    resize_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let network_scroll = widgets.network_scroll.clone();
    let bt_scroll = widgets.bt_scroll.clone();
    let audio_scroll = widgets.audio_scroll.clone();
    let win = widgets.window.clone();

    let expanded = Rc::new(Cell::new(false));

    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        if resize_requested.swap(false, std::sync::atomic::Ordering::Relaxed) {
            let is_expanded = !expanded.get();
            expanded.set(is_expanded);

            let max_h = if is_expanded {
                window::EXPANDED_MAX_LIST_HEIGHT
            } else {
                window::MAX_LIST_HEIGHT
            };

            network_scroll.set_max_content_height(max_h);
            bt_scroll.set_max_content_height(max_h);
            audio_scroll.set_max_content_height(max_h);

            log::info!("Panel resized: expanded={is_expanded} max_height={max_h}");

            if !is_expanded {
                // Shrink window back when collapsing
                win.set_default_size(window::WINDOW_WIDTH, -1);
            }
        }
        glib::ControlFlow::Continue
    });
}

/// Poll the reload_requested flag and reload config/CSS when set.
pub(super) fn setup_reload_on_request(
    widgets: &PanelWidgets,
    state: Rc<RefCell<AppState>>,
    reload_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let list_box = widgets.network_list_box.clone();
    let status = widgets.status_label.clone();
    let window = widgets.window.clone();

    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        if reload_requested.swap(false, std::sync::atomic::Ordering::Relaxed) {
            log::info!("Reload requested - refreshing config, CSS, and network list");
            let state = Rc::clone(&state);
            let list_box = list_box.clone();
            let status = status.clone();
            let window = window.clone();

            glib::spawn_future_local(async move {
                // Reload config and re-apply position/margins
                let config = crate::config::Config::load();
                crate::ui::window::apply_position(&window, &config);
                // Reload CSS
                crate::ui::window::reload_css(&config);
                // Refresh network list (which will reload config for icons)
                refresh_list(&state, &list_box, &status).await;
            });
        }
        glib::ControlFlow::Continue
    });
}
