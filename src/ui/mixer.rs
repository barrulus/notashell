//! Audio mixer panel — three sections: Outputs (sinks), Inputs (sources), Applications.
//!
//! Rows are persistent: once created they're kept in `MixerWidgets` maps keyed
//! by PulseAudio index, and the `sync_*` functions diff incoming state against
//! the current rows, patching in place. This keeps a user's drag on a mixer
//! slider uninterrupted while the footer master slider is also updating, since
//! the slider widget they're holding is never destroyed.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Label, ListBox, Orientation, PolicyType, ScrolledWindow, SelectionMode,
    Widget,
};

use crate::controls::audio::{AudioApp, AudioManager, AudioSink, AudioSource};
use crate::ui::mixer_row::{self, MixerRow, MixerRowCallbacks, MixerRowKind, SinkInfo};
use crate::ui::window::{MAX_LIST_HEIGHT, MIN_LIST_HEIGHT};

pub type MixerRowMap = Rc<RefCell<HashMap<u32, MixerRow>>>;

/// All mixer UI handles needed by the app controller.
pub struct MixerWidgets {
    pub scroll: ScrolledWindow,
    pub sinks_list: ListBox,
    pub sources_list: ListBox,
    pub apps_list: ListBox,
    pub sink_rows: MixerRowMap,
    pub source_rows: MixerRowMap,
    pub app_rows: MixerRowMap,
}

/// Build the mixer layout with three sections inside a scrollable area.
pub fn build_mixer() -> MixerWidgets {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.add_css_class("mixer-container");

    // ── Outputs section ──
    let sinks_header = Label::new(Some("Outputs"));
    sinks_header.add_css_class("mixer-section-header");
    sinks_header.set_halign(gtk4::Align::Start);
    sinks_header.set_margin_start(20);
    sinks_header.set_margin_top(8);
    sinks_header.set_margin_bottom(4);

    let sinks_list = ListBox::new();
    sinks_list.add_css_class("mixer-list");
    sinks_list.set_selection_mode(SelectionMode::None);
    sinks_list.set_activate_on_single_click(true);

    container.append(&sinks_header);
    container.append(&sinks_list);

    // ── Inputs section ──
    let sources_header = Label::new(Some("Inputs"));
    sources_header.add_css_class("mixer-section-header");
    sources_header.set_halign(gtk4::Align::Start);
    sources_header.set_margin_start(20);
    sources_header.set_margin_top(8);
    sources_header.set_margin_bottom(4);

    let sources_list = ListBox::new();
    sources_list.add_css_class("mixer-list");
    sources_list.set_selection_mode(SelectionMode::None);
    sources_list.set_activate_on_single_click(true);

    container.append(&sources_header);
    container.append(&sources_list);

    // ── Applications section ──
    let apps_header = Label::new(Some("Applications"));
    apps_header.add_css_class("mixer-section-header");
    apps_header.set_halign(gtk4::Align::Start);
    apps_header.set_margin_start(20);
    apps_header.set_margin_top(8);
    apps_header.set_margin_bottom(4);

    let apps_list = ListBox::new();
    apps_list.add_css_class("mixer-list");
    apps_list.set_selection_mode(SelectionMode::None);
    apps_list.set_activate_on_single_click(false);

    container.append(&apps_header);
    container.append(&apps_list);

    // Wrap in ScrolledWindow
    let scroll = ScrolledWindow::new();
    scroll.add_css_class("mixer-scroll");
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_has_frame(false);
    scroll.set_min_content_height(MIN_LIST_HEIGHT);
    scroll.set_max_content_height(MAX_LIST_HEIGHT);
    scroll.set_child(Some(&container));

    MixerWidgets {
        scroll,
        sinks_list,
        sources_list,
        apps_list,
        sink_rows: Rc::new(RefCell::new(HashMap::new())),
        source_rows: Rc::new(RefCell::new(HashMap::new())),
        app_rows: Rc::new(RefCell::new(HashMap::new())),
    }
}

/// Diff the incoming sink list against existing rows and patch in place.
pub fn sync_sinks(
    list_box: &ListBox,
    rows: &MixerRowMap,
    sinks: &[AudioSink],
    manager: &Rc<AudioManager>,
) {
    let mut rows = rows.borrow_mut();
    remove_empty_label(list_box);

    if sinks.is_empty() {
        for (_, row) in rows.drain() {
            list_box.remove(&row.widget);
        }
        append_empty_label(list_box, "No output devices");
        return;
    }

    // Drop rows whose PulseAudio index no longer exists.
    let keep: HashSet<u32> = sinks.iter().map(|s| s.index).collect();
    rows.retain(|idx, row| {
        if keep.contains(idx) {
            true
        } else {
            list_box.remove(&row.widget);
            false
        }
    });

    for sink in sinks {
        let icon = sink_icon(sink.is_default);
        let tooltip = if sink.is_default {
            "Default output"
        } else {
            "Click to set as default output"
        };

        if let Some(row) = rows.get(&sink.index) {
            row.update_icon(icon);
            row.update_name(&sink.description);
            row.update_volume(sink.volume_percent);
            row.update_muted(sink.muted);
            row.update_default(sink.is_default);
            row.set_tooltip(Some(tooltip));
        } else {
            let mgr_vol = Rc::clone(manager);
            let mgr_mute = Rc::clone(manager);
            let idx = sink.index;
            let row = mixer_row::build_mixer_row(
                icon,
                &sink.description,
                sink.volume_percent,
                sink.muted,
                sink.is_default,
                MixerRowKind::Sink,
                MixerRowCallbacks {
                    on_volume_changed: Box::new(move |vol| {
                        mgr_vol.set_sink_volume(idx, vol);
                    }),
                    on_mute_toggled: Box::new(move |muted| {
                        mgr_mute.set_sink_mute(idx, muted);
                    }),
                    on_sink_changed: None,
                },
                None,
            );
            row.set_tooltip(Some(tooltip));
            list_box.append(&row.widget);
            rows.insert(sink.index, row);
        }
    }
}

/// Diff the incoming source list against existing rows and patch in place.
pub fn sync_sources(
    list_box: &ListBox,
    rows: &MixerRowMap,
    sources: &[AudioSource],
    manager: &Rc<AudioManager>,
) {
    let mut rows = rows.borrow_mut();
    remove_empty_label(list_box);

    if sources.is_empty() {
        for (_, row) in rows.drain() {
            list_box.remove(&row.widget);
        }
        append_empty_label(list_box, "No input devices");
        return;
    }

    let keep: HashSet<u32> = sources.iter().map(|s| s.index).collect();
    rows.retain(|idx, row| {
        if keep.contains(idx) {
            true
        } else {
            list_box.remove(&row.widget);
            false
        }
    });

    for source in sources {
        let icon = "\u{f130}"; // 🎤
        let tooltip = if source.is_default {
            "Default input"
        } else {
            "Click to set as default input"
        };

        if let Some(row) = rows.get(&source.index) {
            row.update_icon(icon);
            row.update_name(&source.description);
            row.update_volume(source.volume_percent);
            row.update_muted(source.muted);
            row.update_default(source.is_default);
            row.set_tooltip(Some(tooltip));
        } else {
            let mgr_vol = Rc::clone(manager);
            let mgr_mute = Rc::clone(manager);
            let idx = source.index;
            let row = mixer_row::build_mixer_row(
                icon,
                &source.description,
                source.volume_percent,
                source.muted,
                source.is_default,
                MixerRowKind::Source,
                MixerRowCallbacks {
                    on_volume_changed: Box::new(move |vol| {
                        mgr_vol.set_source_volume(idx, vol);
                    }),
                    on_mute_toggled: Box::new(move |muted| {
                        mgr_mute.set_source_mute(idx, muted);
                    }),
                    on_sink_changed: None,
                },
                None,
            );
            row.set_tooltip(Some(tooltip));
            list_box.append(&row.widget);
            rows.insert(source.index, row);
        }
    }
}

/// Diff the incoming apps list against existing rows and patch in place.
pub fn sync_apps(
    list_box: &ListBox,
    rows: &MixerRowMap,
    apps: &[AudioApp],
    sinks: &[AudioSink],
    manager: &Rc<AudioManager>,
) {
    let mut rows = rows.borrow_mut();
    remove_empty_label(list_box);

    if apps.is_empty() {
        for (_, row) in rows.drain() {
            list_box.remove(&row.widget);
        }
        append_empty_label(list_box, "No apps playing");
        return;
    }

    let sink_list: Vec<(u32, String)> = sinks
        .iter()
        .map(|s| (s.index, s.description.clone()))
        .collect();
    let want_dropdown = sink_list.len() > 1;

    let keep: HashSet<u32> = apps.iter().map(|a| a.index).collect();
    rows.retain(|idx, row| {
        if keep.contains(idx) {
            true
        } else {
            list_box.remove(&row.widget);
            false
        }
    });

    // If the dropdown-visibility state has flipped (sinks went from 1 → 2+ or
    // back), the row structure differs and we have to rebuild those rows. In
    // practice this is rare; normal PA traffic just updates volume/mute.
    rows.retain(|_, row| {
        let row_has_dropdown = row.dropdown().is_some();
        if row_has_dropdown != want_dropdown {
            list_box.remove(&row.widget);
            false
        } else {
            true
        }
    });

    for app in apps {
        let icon = "\u{f001}"; // 🎵

        if let Some(row) = rows.get(&app.index) {
            row.update_icon(icon);
            row.update_name(&app.name);
            row.update_volume(app.volume_percent);
            row.update_muted(app.muted);
            row.update_sinks(&sink_list, app.sink_index);
        } else {
            let mgr_vol = Rc::clone(manager);
            let mgr_mute = Rc::clone(manager);
            let mgr_move = Rc::clone(manager);
            let idx = app.index;

            let sink_info = if want_dropdown {
                Some(SinkInfo {
                    sinks: sink_list.clone(),
                    current_sink_index: app.sink_index,
                })
            } else {
                None
            };

            let row = mixer_row::build_mixer_row(
                icon,
                &app.name,
                app.volume_percent,
                app.muted,
                false,
                MixerRowKind::App,
                MixerRowCallbacks {
                    on_volume_changed: Box::new(move |vol| {
                        mgr_vol.set_app_volume(idx, vol);
                    }),
                    on_mute_toggled: Box::new(move |muted| {
                        mgr_mute.set_app_mute(idx, muted);
                    }),
                    on_sink_changed: Some(Box::new(move |sink_idx| {
                        mgr_move.move_app_to_sink(idx, sink_idx);
                    })),
                },
                sink_info,
            );
            list_box.append(&row.widget);
            rows.insert(app.index, row);
        }
    }
}

fn sink_icon(is_default: bool) -> &'static str {
    if is_default { "\u{f028}" } else { "\u{f025}" }
}

fn append_empty_label(list_box: &ListBox, text: &str) {
    let label = Label::new(Some(text));
    label.add_css_class("empty-label");
    list_box.append(&label);
}

/// Remove any direct-child empty-state label from the listbox.
/// Leaves rows alone (rows are managed by the MixerRowMap).
fn remove_empty_label(list_box: &ListBox) {
    let mut child = list_box.first_child();
    while let Some(c) = child {
        let next = c.next_sibling();
        if let Some(label) = c.downcast_ref::<Label>() {
            if label.has_css_class("empty-label") {
                list_box.remove(&c as &Widget);
            }
        }
        child = next;
    }
}
