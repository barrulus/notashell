//! Single mixer row widget — icon + name + volume slider + mute button.
//!
//! Rows are long-lived: rather than being rebuilt every time PulseAudio fires
//! a subscription event, they expose `update_*` methods so the sync layer can
//! patch state in place. This is what lets the user keep dragging a slider
//! while the footer master volume also moves — tearing rows down on every
//! event destroyed the drag target mid-motion.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DropDown, Label, ListBoxRow, Orientation, Scale, StringList,
};

/// The kind of audio item this row represents.
#[derive(Clone, Copy, PartialEq, Eq)]
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

/// A persistent mixer row whose widgets can be updated from PulseAudio state
/// without being torn down.
pub struct MixerRow {
    pub widget: ListBoxRow,
    hbox: GtkBox,
    icon_label: Label,
    name_label: Label,
    scale: Scale,
    scale_handler: glib::SignalHandlerId,
    mute_btn: Button,
    mute_state: Rc<Cell<bool>>,
    is_default: Cell<bool>,
    // App rows only: dropdown + its model + last-known sink list for diffing.
    dropdown: Option<DropDown>,
    dropdown_handler: Option<glib::SignalHandlerId>,
    dropdown_indices: Rc<std::cell::RefCell<Vec<u32>>>,
    dropdown_labels: Rc<std::cell::RefCell<Vec<String>>>,
    vbox: Option<GtkBox>,
}

impl MixerRow {
    /// Update the volume scale value without re-firing the user callback.
    pub fn update_volume(&self, percent: f64) {
        // Avoid triggering the user-drag callback on programmatic updates.
        // Also avoids the cosmetic jitter of the scale snapping back while the
        // user is mid-drag on a different row.
        self.scale.block_signal(&self.scale_handler);
        self.scale.set_value(percent);
        self.scale.unblock_signal(&self.scale_handler);
    }

    pub fn update_muted(&self, muted: bool) {
        if self.mute_state.get() == muted {
            return;
        }
        self.mute_state.set(muted);
        update_mute_icon(&self.mute_btn, muted);
        if muted {
            self.scale.add_css_class("muted");
        } else {
            self.scale.remove_css_class("muted");
        }
    }

    pub fn update_default(&self, is_default: bool) {
        if self.is_default.get() == is_default {
            return;
        }
        self.is_default.set(is_default);
        if is_default {
            self.widget.add_css_class("default");
        } else {
            self.widget.remove_css_class("default");
        }
    }

    pub fn update_icon(&self, icon: &str) {
        if self.icon_label.text().as_str() != icon {
            self.icon_label.set_text(icon);
        }
    }

    /// Returns `Some(&DropDown)` for app rows that have a sink selector.
    pub fn dropdown(&self) -> Option<&DropDown> {
        self.dropdown.as_ref()
    }

    pub fn update_name(&self, name: &str) {
        if self.name_label.text().as_str() != name {
            self.name_label.set_text(name);
        }
    }

    /// Set tooltip on the row and the widgets most likely to sit under the
    /// pointer. GTK4 queries the tooltip on the focused/hovered widget
    /// directly, so setting only on the row makes the tooltip never fire when
    /// the pointer is over the icon or name. The scale and mute button are
    /// deliberately skipped so they can keep their own semantic hovers.
    pub fn set_tooltip(&self, text: Option<&str>) {
        self.widget.set_tooltip_text(text);
        self.hbox.set_tooltip_text(text);
        self.icon_label.set_tooltip_text(text);
        self.name_label.set_tooltip_text(text);
    }

    /// For app rows: update the dropdown's sink list and selection.
    /// No-op if this row has no dropdown or nothing meaningful changed.
    pub fn update_sinks(&self, sinks: &[(u32, String)], current_sink_index: u32) {
        let vbox = match &self.vbox {
            Some(v) => v,
            None => return,
        };

        let should_have = sinks.len() > 1;

        // Labels/indices changed?
        let labels_changed = {
            let old = self.dropdown_labels.borrow();
            old.len() != sinks.len()
                || old.iter().zip(sinks.iter()).any(|(a, b)| *a != b.1)
        };
        let indices_changed = {
            let old = self.dropdown_indices.borrow();
            old.len() != sinks.len()
                || old.iter().zip(sinks.iter()).any(|(a, b)| *a != b.0)
        };

        if should_have && self.dropdown.is_none() {
            // Would need to rebuild from scratch to add a dropdown; this path
            // is hit by the sync layer which will drop and recreate the row.
            return;
        }

        if !should_have && self.dropdown.is_some() {
            if let Some(dd) = &self.dropdown {
                vbox.remove(dd);
            }
            // We can't rewrite self.dropdown here (not &mut self); the sync
            // layer drops and recreates the row when the dropdown-visibility
            // state flips, so this branch is mostly defensive.
            return;
        }

        let dropdown = match &self.dropdown {
            Some(dd) => dd,
            None => return,
        };
        let handler = match &self.dropdown_handler {
            Some(h) => h,
            None => return,
        };

        if labels_changed {
            let descriptions: Vec<&str> = sinks.iter().map(|s| s.1.as_str()).collect();
            let string_list = StringList::new(&descriptions);
            dropdown.set_model(Some(&string_list));
            *self.dropdown_labels.borrow_mut() =
                sinks.iter().map(|(_, d)| d.clone()).collect();
        }
        if indices_changed {
            *self.dropdown_indices.borrow_mut() = sinks.iter().map(|(i, _)| *i).collect();
        }

        let selected = self
            .dropdown_indices
            .borrow()
            .iter()
            .position(|i| *i == current_sink_index)
            .unwrap_or(0);
        if dropdown.selected() as usize != selected {
            dropdown.block_signal(handler);
            dropdown.set_selected(selected as u32);
            dropdown.unblock_signal(handler);
        }
    }

}

/// Build a mixer row. See the struct doc for how updates flow.
pub fn build_mixer_row(
    icon: &str,
    name: &str,
    volume_percent: f64,
    muted: bool,
    is_default: bool,
    _kind: MixerRowKind,
    callbacks: MixerRowCallbacks,
    sink_info: Option<SinkInfo>,
) -> MixerRow {
    let row = ListBoxRow::new();
    row.add_css_class("mixer-row");
    row.set_activatable(true);

    if is_default {
        row.add_css_class("default");
    }

    // Outer spacing comes from `.mixer-row`'s CSS padding; no extra inner
    // hbox margins here.
    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.add_css_class("mixer-row-content");

    let has_dropdown = sink_info
        .as_ref()
        .is_some_and(|info| info.sinks.len() > 1);

    // When the app sink dropdown is present, the row is two lines; top-align
    // the other children with the name rather than float between the lines.
    let child_valign = if has_dropdown {
        gtk4::Align::Start
    } else {
        gtk4::Align::Center
    };

    let icon_label = Label::new(Some(icon));
    icon_label.add_css_class("mixer-icon");
    icon_label.set_valign(child_valign);

    let name_label = Label::new(Some(name));
    name_label.add_css_class("mixer-name");
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_width_chars(10);
    name_label.set_max_width_chars(14);

    let mut dropdown_out: Option<DropDown> = None;
    let mut dropdown_handler_out: Option<glib::SignalHandlerId> = None;
    let dropdown_indices = Rc::new(std::cell::RefCell::new(Vec::<u32>::new()));
    let dropdown_labels = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let mut vbox_out: Option<GtkBox> = None;

    let name_area: gtk4::Widget = if let Some(info) = &sink_info {
        let vbox = GtkBox::new(Orientation::Vertical, 2);
        vbox.set_valign(gtk4::Align::Start);
        vbox.append(&name_label);

        if info.sinks.len() > 1 {
            let descriptions: Vec<&str> =
                info.sinks.iter().map(|s| s.1.as_str()).collect();
            let string_list = StringList::new(&descriptions);
            let dropdown = DropDown::new(Some(string_list), gtk4::Expression::NONE);
            dropdown.add_css_class("mixer-sink-dropdown");
            dropdown.set_tooltip_text(Some("Route this app to a different output"));

            let selected = info
                .sinks
                .iter()
                .position(|(idx, _)| *idx == info.current_sink_index)
                .unwrap_or(0);
            dropdown.set_selected(selected as u32);

            *dropdown_indices.borrow_mut() =
                info.sinks.iter().map(|(i, _)| *i).collect();
            *dropdown_labels.borrow_mut() =
                info.sinks.iter().map(|(_, d)| d.clone()).collect();

            let handler = if let Some(on_sink) = callbacks.on_sink_changed {
                let on_sink = Rc::new(on_sink);
                let indices = Rc::clone(&dropdown_indices);
                dropdown.connect_selected_notify(move |dd| {
                    let pos = dd.selected() as usize;
                    if let Some(&sink_idx) = indices.borrow().get(pos) {
                        (on_sink)(sink_idx);
                    }
                })
            } else {
                // Install a no-op handler so we always have something to block
                // when programmatically updating the selection.
                dropdown.connect_selected_notify(|_| {})
            };

            vbox.append(&dropdown);

            dropdown_out = Some(dropdown);
            dropdown_handler_out = Some(handler);
        }

        vbox_out = Some(vbox.clone());
        vbox.upcast()
    } else {
        name_label.set_valign(gtk4::Align::Center);
        name_label.clone().upcast()
    };

    // Volume slider
    let scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_value(volume_percent);
    scale.add_css_class("mixer-scale");
    scale.set_hexpand(true);
    scale.set_valign(child_valign);
    scale.set_draw_value(false);

    if muted {
        scale.add_css_class("muted");
    }

    // Mute button
    let mute_btn = Button::new();
    mute_btn.add_css_class("mixer-mute-btn");
    mute_btn.add_css_class("flat");
    mute_btn.set_valign(child_valign);
    update_mute_icon(&mute_btn, muted);

    // Wire volume slider. No debounce: rows are updated in place rather than
    // rebuilt, so PulseAudio events can't destroy the widget under the user's
    // finger. Feedback is suppressed via `block_signal` in update_volume.
    let on_vol = callbacks.on_volume_changed;
    let scale_handler = scale.connect_value_changed(move |s| {
        (on_vol)(s.value());
    });

    // Wire mute button
    let mute_state = Rc::new(Cell::new(muted));
    let scale_mute = scale.clone();
    let on_mute = callbacks.on_mute_toggled;
    let mute_state_cb = Rc::clone(&mute_state);
    mute_btn.connect_clicked(move |btn| {
        let new_muted = !mute_state_cb.get();
        mute_state_cb.set(new_muted);
        update_mute_icon(btn, new_muted);
        if new_muted {
            scale_mute.add_css_class("muted");
        } else {
            scale_mute.remove_css_class("muted");
        }
        (on_mute)(new_muted);
    });

    hbox.append(&icon_label);
    hbox.append(&name_area);
    hbox.append(&scale);
    hbox.append(&mute_btn);

    row.set_child(Some(&hbox));

    MixerRow {
        widget: row,
        hbox,
        icon_label,
        name_label,
        scale,
        scale_handler,
        mute_btn,
        mute_state,
        is_default: Cell::new(is_default),
        dropdown: dropdown_out,
        dropdown_handler: dropdown_handler_out,
        dropdown_indices,
        dropdown_labels,
        vbox: vbox_out,
    }
}

fn update_mute_icon(btn: &Button, muted: bool) {
    btn.set_icon_name(if muted {
        "audio-volume-muted-symbolic"
    } else {
        "audio-volume-high-symbolic"
    });
}
