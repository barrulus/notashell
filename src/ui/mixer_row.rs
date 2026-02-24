//! Single mixer row widget — icon + name + volume slider + mute button.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, ListBoxRow, Orientation, Scale,
};

/// The kind of audio item this row represents.
#[derive(Clone, Copy)]
pub enum MixerRowKind {
    Sink,
    Source,
    App,
}

/// Callbacks for mixer row interactions.
pub struct MixerRowCallbacks {
    pub on_volume_changed: Box<dyn Fn(f64)>,
    pub on_mute_toggled: Box<dyn Fn(bool)>,
}

/// Build a `ListBoxRow` for a single audio device or application.
pub fn build_mixer_row(
    icon: &str,
    name: &str,
    volume_percent: f64,
    muted: bool,
    is_default: bool,
    _kind: MixerRowKind,
    callbacks: MixerRowCallbacks,
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

    // Name
    let name_label = Label::new(Some(name));
    name_label.add_css_class("mixer-name");
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_valign(gtk4::Align::Center);
    name_label.set_width_chars(10);
    name_label.set_max_width_chars(14);

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
    hbox.append(&name_label);
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
