//! Audio mixer controller — tab activation, refresh, live updates.
//!
//! Follows the Bluetooth controller pattern: async init with graceful fallback.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::controls::audio::AudioManager;
use crate::ui::mixer;
use crate::ui::window::PanelWidgets;

use super::AppState;

/// Set up all Audio mixer UI event handlers.
///
/// If PulseAudio is unavailable, the audio tab is hidden entirely.
pub(super) fn setup_audio(widgets: &PanelWidgets, state: Rc<RefCell<AppState>>) {
    let audio_tab = widgets.audio_tab.clone();
    let sinks_list = widgets.audio_sinks_list.clone();
    let sources_list = widgets.audio_sources_list.clone();
    let apps_list = widgets.audio_apps_list.clone();
    let audio_scroll = widgets.audio_scroll.clone();
    let status = widgets.status_label.clone();
    let switch = widgets.wifi_switch.clone();
    let scan_btn = widgets.scan_button.clone();
    let title = widgets.title_label.clone();

    // ── Audio tab activation: sync header and populate mixer ──
    {
        let state = Rc::clone(&state);
        let sinks_list = sinks_list.clone();
        let sources_list = sources_list.clone();
        let apps_list = apps_list.clone();
        let audio_scroll = audio_scroll.clone();
        let status = status.clone();
        let switch = switch.clone();
        let scan_btn = scan_btn.clone();
        let title = title.clone();

        audio_tab.connect_toggled(move |btn| {
            if !btn.is_active() {
                return;
            }

            title.set_text("Audio");
            scan_btn.set_tooltip_text(Some("Refresh audio devices"));
            switch.set_visible(false); // No power toggle for audio
            status.set_text("Audio Mixer");

            let state = Rc::clone(&state);
            let sinks_list = sinks_list.clone();
            let sources_list = sources_list.clone();
            let apps_list = apps_list.clone();
            let audio_scroll = audio_scroll.clone();

            audio_scroll.set_visible(true);
            refresh_mixer(&state, &sinks_list, &sources_list, &apps_list);
        });
    }

    // ── Sink row click: set as default sink ──
    {
        let state_c = Rc::clone(&state);
        let sinks_list_c = sinks_list.clone();
        let sources_list_c = sources_list.clone();
        let apps_list_c = apps_list.clone();

        sinks_list.connect_row_activated(move |_list, row| {
            let index = row.index() as usize;
            let st = state_c.borrow();
            if let Some(mgr) = &st.audio {
                if let Some(sink) = st.audio_sinks.get(index) {
                    if !sink.is_default {
                        mgr.set_default_sink(&sink.name);
                        // Refresh will happen via on_change subscription
                    }
                }
            }
            drop(st);
            refresh_mixer(&state_c, &sinks_list_c, &sources_list_c, &apps_list_c);
        });
    }

    // ── Source row click: set as default source ──
    {
        let state_c = Rc::clone(&state);
        let sinks_list_c = sinks_list.clone();
        let sources_list_c = sources_list.clone();
        let apps_list_c = apps_list.clone();

        sources_list.connect_row_activated(move |_list, row| {
            let index = row.index() as usize;
            let st = state_c.borrow();
            if let Some(mgr) = &st.audio {
                if let Some(source) = st.audio_sources.get(index) {
                    if !source.is_default {
                        mgr.set_default_source(&source.name);
                    }
                }
            }
            drop(st);
            refresh_mixer(&state_c, &sinks_list_c, &sources_list_c, &apps_list_c);
        });
    }

    // ── Initialize AudioManager ──
    {
        let state = Rc::clone(&state);
        let audio_tab = widgets.audio_tab.clone();
        let sinks_list = sinks_list.clone();
        let sources_list = sources_list.clone();
        let apps_list = apps_list.clone();

        // on_change callback: refresh mixer when PA events fire (only if audio tab active)
        let audio_tab_change = audio_tab.clone();
        let state_change = Rc::clone(&state);
        let sinks_list_change = sinks_list.clone();
        let sources_list_change = sources_list.clone();
        let apps_list_change = apps_list.clone();

        let on_change = move || {
            if audio_tab_change.is_active() {
                refresh_mixer(
                    &state_change,
                    &sinks_list_change,
                    &sources_list_change,
                    &apps_list_change,
                );
            }
        };

        match AudioManager::new(
            on_change,
            move |result| match result {
                Ok(()) => {
                    log::info!("Audio mixer connected successfully");
                }
                Err(e) => {
                    log::error!("Audio mixer connection failed: {e}");
                    audio_tab.set_visible(false);
                }
            },
        ) {
            Ok(manager) => {
                state.borrow_mut().audio = Some(manager);
            }
            Err(e) => {
                log::error!("Failed to create AudioManager: {e}");
                widgets.audio_tab.set_visible(false);
            }
        }
    }
}

/// Refresh all three mixer sections from PulseAudio state.
fn refresh_mixer(
    state: &Rc<RefCell<AppState>>,
    sinks_list: &gtk4::ListBox,
    sources_list: &gtk4::ListBox,
    apps_list: &gtk4::ListBox,
) {
    let mgr = match state.borrow().audio.clone() {
        Some(m) => m,
        None => return,
    };

    // Fetch sinks
    {
        let state = Rc::clone(state);
        let sinks_list = sinks_list.clone();
        let mgr_clone = Rc::clone(&mgr);
        mgr.get_sinks(move |sinks| {
            mixer::populate_sinks(&sinks_list, &sinks, &mgr_clone);
            state.borrow_mut().audio_sinks = sinks;
        });
    }

    // Fetch sources
    {
        let state = Rc::clone(state);
        let sources_list = sources_list.clone();
        let mgr_clone = Rc::clone(&mgr);
        mgr.get_sources(move |sources| {
            mixer::populate_sources(&sources_list, &sources, &mgr_clone);
            state.borrow_mut().audio_sources = sources;
        });
    }

    // Fetch apps
    {
        let state = Rc::clone(state);
        let apps_list = apps_list.clone();
        let mgr_clone = Rc::clone(&mgr);
        mgr.get_apps(move |apps| {
            mixer::populate_apps(&apps_list, &apps, &mgr_clone);
            state.borrow_mut().audio_apps = apps;
        });
    }
}

/// Wire the scan button to refresh audio when on the audio tab.
pub(super) fn setup_audio_scan_button(widgets: &PanelWidgets, state: Rc<RefCell<AppState>>) {
    let audio_tab = widgets.audio_tab.clone();
    let sinks_list = widgets.audio_sinks_list.clone();
    let sources_list = widgets.audio_sources_list.clone();
    let apps_list = widgets.audio_apps_list.clone();
    let scan_btn = widgets.scan_button.clone();

    scan_btn.connect_clicked(move |_btn| {
        if !audio_tab.is_active() {
            return;
        }
        refresh_mixer(&state, &sinks_list, &sources_list, &apps_list);
    });
}

/// Restore the switch visibility when leaving the audio tab.
pub(super) fn setup_audio_tab_leave(widgets: &PanelWidgets) {
    let switch = widgets.wifi_switch.clone();
    let wifi_tab = widgets.wifi_tab.clone();
    let bt_tab = widgets.bt_tab.clone();

    // When WiFi tab activated, show switch again
    {
        let switch = switch.clone();
        wifi_tab.connect_toggled(move |btn| {
            if btn.is_active() {
                switch.set_visible(true);
            }
        });
    }
    {
        let switch = switch.clone();
        bt_tab.connect_toggled(move |btn| {
            if btn.is_active() {
                switch.set_visible(true);
            }
        });
    }
}
