//! Single mixer row widget — icon + name + volume slider + mute button.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Label, ListBoxRow, Orientation, Scale, StringList,
};

/// The kind of audio item this row represents.
#[derive(Clone, Copy)]
pub enum MixerRowKind {
    Sink,
    Source,
    App,
}

/// Sink info for the dropdown: (index, description) pairs.
pub struct SinkInfo {
    pub sinks: Vec<(u32, String)>,
    pub current_sink_index: u32,
}

/// Callbacks for mixer row interactions.
pub struct MixerRowCallbacks {
    pub on_volume_changed: Box<dyn Fn(f64)>,
    pub on_mute_toggled: Box<dyn Fn(bool)>,
    pub on_sink_changed: Option<Box<dyn Fn(u32)>>,
}

/// Build a `ListBoxRow` for a single audio device or application.
///
/// When `sink_info` is provided (for App rows), a sink selector dropdown is
/// shown below the app name so users can reroute the stream.
pub fn build_mixer_row(
    icon: &str,
    name: &str,
    volume_percent: f64,
    muted: bool,
    is_default: bool,
    _kind: MixerRowKind,
    callbacks: MixerRowCallbacks,
    sink_info: Option<SinkInfo>,
) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.add_css_class("mixer-row");
    row.set_activatable(true);

    if is_default {
        row.add_css_class("default");
    }

    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.add_css_class("mixer-row-content");
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);

    // Icon
    let icon_label = Label::new(Some(icon));
    icon_label.add_css_class("mixer-icon");
    icon_label.set_valign(gtk4::Align::Center);

    // Name (+ optional sink dropdown for App rows)
    let name_label = Label::new(Some(name));
    name_label.add_css_class("mixer-name");
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_width_chars(10);
    name_label.set_max_width_chars(14);

    let name_area: gtk4::Widget = if let Some(info) = &sink_info {
        let vbox = GtkBox::new(Orientation::Vertical, 2);
        vbox.set_valign(gtk4::Align::Center);
        name_label.set_valign(gtk4::Align::Start);
        vbox.append(&name_label);

        if info.sinks.len() > 1 {
            let descriptions: Vec<&str> = info.sinks.iter().map(|s| s.1.as_str()).collect();
            let string_list = StringList::new(&descriptions);
            let dropdown = DropDown::new(Some(string_list), gtk4::Expression::NONE);
            dropdown.add_css_class("mixer-sink-dropdown");

            // Select the current sink
            let selected = info
                .sinks
                .iter()
                .position(|(idx, _)| *idx == info.current_sink_index)
                .unwrap_or(0);
            dropdown.set_selected(selected as u32);

            // Wire the callback
            if let Some(on_sink) = callbacks.on_sink_changed {
                let on_sink = Rc::new(on_sink);
                let sink_indices: Vec<u32> = info.sinks.iter().map(|(idx, _)| *idx).collect();
                dropdown.connect_selected_notify(move |dd| {
                    let pos = dd.selected() as usize;
                    if let Some(&sink_idx) = sink_indices.get(pos) {
                        (on_sink)(sink_idx);
                    }
                });
            }

            vbox.append(&dropdown);
        }

        vbox.upcast()
    } else {
        name_label.set_valign(gtk4::Align::Center);
        name_label.upcast()
    };

    // Volume slider
    let scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_value(volume_percent);
    scale.add_css_class("mixer-scale");
    scale.set_hexpand(true);
    scale.set_valign(gtk4::Align::Center);
    scale.set_draw_value(false);

    // Mute button
    let mute_btn = Button::new();
    mute_btn.add_css_class("mixer-mute-btn");
    mute_btn.add_css_class("flat");
    mute_btn.set_valign(gtk4::Align::Center);
    update_mute_icon(&mute_btn, muted);

    if muted {
        scale.add_css_class("muted");
    }

    // Wire volume slider
    let on_vol = Rc::new(callbacks.on_volume_changed);
    let is_updating = Rc::new(Cell::new(false));
    let is_updating_clone = Rc::clone(&is_updating);
    let on_vol_clone = Rc::clone(&on_vol);
    scale.connect_value_changed(move |s| {
        if !is_updating_clone.get() {
            (on_vol_clone)(s.value());
        }
    });

    // Wire mute button
    let on_mute = Rc::new(callbacks.on_mute_toggled);
    let mute_state = Rc::new(Cell::new(muted));
    let scale_clone = scale.clone();
    mute_btn.connect_clicked(move |btn| {
        let new_muted = !mute_state.get();
        mute_state.set(new_muted);
        update_mute_icon(btn, new_muted);
        if new_muted {
            scale_clone.add_css_class("muted");
        } else {
            scale_clone.remove_css_class("muted");
        }
        (on_mute)(new_muted);
    });

    hbox.append(&icon_label);
    hbox.append(&name_area);
    hbox.append(&scale);
    hbox.append(&mute_btn);

    row.set_child(Some(&hbox));
    row
}

fn update_mute_icon(btn: &Button, muted: bool) {
    btn.set_icon_name(if muted {
        "audio-volume-muted-symbolic"
    } else {
        "audio-volume-high-symbolic"
    });
}
