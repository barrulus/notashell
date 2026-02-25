//! Audio mixer panel — three sections: Outputs (sinks), Inputs (sources), Applications.

use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Label, ListBox, Orientation, PolicyType, ScrolledWindow, SelectionMode,
};

use crate::controls::audio::{AudioApp, AudioManager, AudioSink, AudioSource};
use crate::ui::mixer_row::{self, MixerRowCallbacks, MixerRowKind, SinkInfo};
use crate::ui::window::{MAX_LIST_HEIGHT, MIN_LIST_HEIGHT};

/// All mixer UI handles needed by the app controller.
pub struct MixerWidgets {
    pub scroll: ScrolledWindow,
    pub sinks_list: ListBox,
    pub sources_list: ListBox,
    pub apps_list: ListBox,
}

/// Build the mixer layout with three sections inside a scrollable area.
pub fn build_mixer() -> MixerWidgets {
    let container = GtkBox::new(Orientation::Vertical, 0);
    container.add_css_class("mixer-container");

    // ── Outputs section ──
    let sinks_header = Label::new(Some("Outputs"));
    sinks_header.add_css_class("mixer-section-header");
    sinks_header.set_halign(gtk4::Align::Start);
    sinks_header.set_margin_start(12);
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
    sources_header.set_margin_start(12);
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
    apps_header.set_margin_start(12);
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
    }
}

/// Clear and repopulate the sinks list.
pub fn populate_sinks(list_box: &ListBox, sinks: &[AudioSink], manager: &Rc<AudioManager>) {
    clear_list(list_box);

    if sinks.is_empty() {
        let empty = Label::new(Some("No output devices"));
        empty.add_css_class("empty-label");
        list_box.append(&empty);
        return;
    }

    for sink in sinks {
        let icon = if sink.is_default { "\u{f028}" } else { "\u{f025}" }; // 🔊 / 🔉
        let mgr = Rc::clone(manager);
        let idx = sink.index;
        let mgr2 = Rc::clone(manager);

        let row = mixer_row::build_mixer_row(
            icon,
            &sink.description,
            sink.volume_percent,
            sink.muted,
            sink.is_default,
            MixerRowKind::Sink,
            MixerRowCallbacks {
                on_volume_changed: Box::new(move |vol| {
                    mgr.set_sink_volume(idx, vol);
                }),
                on_mute_toggled: Box::new(move |muted| {
                    mgr2.set_sink_mute(idx, muted);
                }),
                on_sink_changed: None,
            },
            None,
        );
        list_box.append(&row);
    }
}

/// Clear and repopulate the sources list.
pub fn populate_sources(
    list_box: &ListBox,
    sources: &[AudioSource],
    manager: &Rc<AudioManager>,
) {
    clear_list(list_box);

    if sources.is_empty() {
        let empty = Label::new(Some("No input devices"));
        empty.add_css_class("empty-label");
        list_box.append(&empty);
        return;
    }

    for source in sources {
        let icon = "\u{f130}"; // 🎤
        let mgr = Rc::clone(manager);
        let idx = source.index;
        let mgr2 = Rc::clone(manager);

        let row = mixer_row::build_mixer_row(
            icon,
            &source.description,
            source.volume_percent,
            source.muted,
            source.is_default,
            MixerRowKind::Source,
            MixerRowCallbacks {
                on_volume_changed: Box::new(move |vol| {
                    mgr.set_source_volume(idx, vol);
                }),
                on_mute_toggled: Box::new(move |muted| {
                    mgr2.set_source_mute(idx, muted);
                }),
                on_sink_changed: None,
            },
            None,
        );
        list_box.append(&row);
    }
}

/// Clear and repopulate the apps list.
pub fn populate_apps(
    list_box: &ListBox,
    apps: &[AudioApp],
    sinks: &[AudioSink],
    manager: &Rc<AudioManager>,
) {
    clear_list(list_box);

    if apps.is_empty() {
        let empty = Label::new(Some("No apps playing"));
        empty.add_css_class("empty-label");
        list_box.append(&empty);
        return;
    }

    let sink_list: Vec<(u32, String)> = sinks
        .iter()
        .map(|s| (s.index, s.description.clone()))
        .collect();

    for app in apps {
        let icon = "\u{f001}"; // 🎵
        let mgr = Rc::clone(manager);
        let idx = app.index;
        let mgr2 = Rc::clone(manager);
        let mgr3 = Rc::clone(manager);

        let sink_info = if sink_list.len() > 1 {
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
                    mgr.set_app_volume(idx, vol);
                }),
                on_mute_toggled: Box::new(move |muted| {
                    mgr2.set_app_mute(idx, muted);
                }),
                on_sink_changed: Some(Box::new(move |sink_idx| {
                    mgr3.move_app_to_sink(idx, sink_idx);
                })),
            },
            sink_info,
        );
        list_box.append(&row);
    }
}

fn clear_list(list_box: &ListBox) {
    while let Some(row) = list_box.first_child() {
        list_box.remove(&row);
    }
}
