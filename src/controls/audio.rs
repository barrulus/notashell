use std::cell::RefCell;
use std::rc::Rc;

use gtk4::glib;
use log::{error, info};

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::subscribe::{Facility, InterestMaskSet, Operation};
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State};
use libpulse_binding::proplist::Proplist;
use libpulse_binding::volume::Volume;
use libpulse_glib_binding::Mainloop;

#[derive(Clone, Debug)]
pub struct AudioSink {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub volume_percent: f64,
    pub muted: bool,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub struct AudioSource {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub volume_percent: f64,
    pub muted: bool,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AudioApp {
    pub index: u32,
    pub name: String,
    pub sink_index: u32,
    pub volume_percent: f64,
    pub muted: bool,
}

/// Manages a PulseAudio connection for full mixer introspection.
/// The `mainloop` field must be retained to keep the GLib integration alive.
#[allow(dead_code)]
pub struct AudioManager {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    default_sink_name: Rc<RefCell<Option<String>>>,
    default_source_name: Rc<RefCell<Option<String>>>,
    on_change: Rc<dyn Fn()>,
}

impl AudioManager {
    pub fn new<F, C>(on_change: F, on_connected: C) -> Result<Rc<Self>, String>
    where
        F: Fn() + 'static,
        C: FnOnce(Result<(), String>) + 'static,
    {
        let mut proplist = Proplist::new().ok_or("Failed to create PulseAudio proplist")?;
        proplist
            .set_str(
                libpulse_binding::proplist::properties::APPLICATION_NAME,
                "notashell-mixer",
            )
            .map_err(|_| "Failed to set application name in proplist")?;

        let mainloop =
            Mainloop::new(None).ok_or("Failed to create PulseAudio GLib mainloop for mixer")?;

        let context = Context::new_with_proplist(&mainloop, "notashell-mixer-context", &proplist)
            .ok_or("Failed to create PulseAudio mixer context")?;

        let manager = Rc::new(Self {
            mainloop: Rc::new(RefCell::new(mainloop)),
            context: Rc::new(RefCell::new(context)),
            default_sink_name: Rc::new(RefCell::new(None)),
            default_source_name: Rc::new(RefCell::new(None)),
            on_change: Rc::new(on_change),
        });

        manager
            .context
            .borrow_mut()
            .connect(None, ContextFlagSet::NOFLAGS, None)
            .map_err(|e| format!("PulseAudio mixer connect error: {}", e))?;

        let mgr_clone = Rc::downgrade(&manager);
        let retry_count = Rc::new(RefCell::new(0u32));
        let on_connected_cb = Rc::new(RefCell::new(Some(on_connected)));
        const MAX_RETRIES: u32 = 50;

        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let mut on_connected_cb = on_connected_cb.borrow_mut();
            if let Some(mgr) = mgr_clone.upgrade() {
                *retry_count.borrow_mut() += 1;
                if *retry_count.borrow() > MAX_RETRIES {
                    let err = "PulseAudio mixer context connection timed out".to_string();
                    error!("{}", err);
                    if let Some(cb) = on_connected_cb.take() {
                        cb(Err(err));
                    }
                    return glib::ControlFlow::Break;
                }
                let state = mgr.context.borrow().get_state();
                if state == State::Ready {
                    mgr.setup();
                    if let Some(cb) = on_connected_cb.take() {
                        cb(Ok(()));
                    }
                    glib::ControlFlow::Break
                } else if state == State::Failed || state == State::Terminated {
                    let err = "PulseAudio mixer context failed or terminated".to_string();
                    error!("{}", err);
                    if let Some(cb) = on_connected_cb.take() {
                        cb(Err(err));
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            } else {
                if let Some(cb) = on_connected_cb.take() {
                    cb(Err(
                        "AudioManager dropped before connection completed".to_string(),
                    ));
                }
                glib::ControlFlow::Break
            }
        });

        Ok(manager)
    }

    fn setup(self: &Rc<Self>) {
        info!("PulseAudio mixer context ready. Setting up subscriptions...");

        let mgr_weak = Rc::downgrade(self);
        // Debounce: track pending refresh source
        let pending_refresh: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

        let mut ctx = self.context.borrow_mut();
        ctx.set_subscribe_callback(Some(Box::new(move |fac, op, _idx| {
            let dominated = matches!(
                fac,
                Some(Facility::Sink)
                    | Some(Facility::Source)
                    | Some(Facility::SinkInput)
                    | Some(Facility::Server)
            );
            // Ignore REMOVE events for sink inputs (app closed) — we still refresh
            let dominated = dominated
                || (fac == Some(Facility::SinkInput) && op == Some(Operation::Removed));

            if !dominated {
                return;
            }

            if let Some(mgr) = mgr_weak.upgrade() {
                // Debounce 100ms
                if let Some(source_id) = pending_refresh.borrow_mut().take() {
                    source_id.remove();
                }
                let on_change = Rc::clone(&mgr.on_change);
                let pending_clone = Rc::clone(&pending_refresh);
                let new_source = glib::timeout_add_local(
                    std::time::Duration::from_millis(100),
                    move || {
                        pending_clone.borrow_mut().take();
                        (on_change)();
                        glib::ControlFlow::Break
                    },
                );
                *pending_refresh.borrow_mut() = Some(new_source);
            }
        })));

        ctx.subscribe(
            InterestMaskSet::SINK
                | InterestMaskSet::SOURCE
                | InterestMaskSet::SINK_INPUT
                | InterestMaskSet::SERVER,
            |success| {
                if !success {
                    error!("Failed to subscribe to PulseAudio mixer events");
                }
            },
        );
    }

    /// Fetch default sink/source names from the server, then fetch all sinks.
    pub fn get_sinks(self: &Rc<Self>, callback: impl FnOnce(Vec<AudioSink>) + 'static) {
        let mgr_weak = Rc::downgrade(self);
        let ctx = self.context.borrow();
        let intro = ctx.introspect();
        let callback = Rc::new(RefCell::new(Some(callback)));

        intro.get_server_info(move |info| {
            let mgr = match mgr_weak.upgrade() {
                Some(m) => m,
                None => {
                    if let Some(cb) = callback.borrow_mut().take() {
                        cb(Vec::new());
                    }
                    return;
                }
            };

            let default_name = info
                .default_sink_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_default();
            *mgr.default_sink_name.borrow_mut() = Some(default_name.clone());

            if let Some(src_name) = info.default_source_name.as_ref() {
                *mgr.default_source_name.borrow_mut() = Some(src_name.to_string());
            }

            let sinks: Rc<RefCell<Vec<AudioSink>>> = Rc::new(RefCell::new(Vec::new()));
            let sinks_clone = Rc::clone(&sinks);
            let callback_clone = Rc::clone(&callback);

            let ctx2 = mgr.context.borrow();
            let intro2 = ctx2.introspect();
            intro2.get_sink_info_list(move |res| match res {
                ListResult::Item(sink) => {
                    let avg_vol = sink.volume.avg();
                    let percent =
                        ((avg_vol.0 as f64 / Volume::NORMAL.0 as f64) * 100.0).min(150.0);
                    let name = sink
                        .name
                        .as_ref()
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    sinks_clone.borrow_mut().push(AudioSink {
                        index: sink.index,
                        name: name.clone(),
                        description: sink
                            .description
                            .as_ref()
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| name),
                        volume_percent: percent,
                        muted: sink.mute,
                        is_default: sink
                            .name
                            .as_ref()
                            .is_some_and(|n| n.to_string() == default_name),
                    });
                }
                ListResult::End => {
                    if let Some(cb) = callback_clone.borrow_mut().take() {
                        cb(sinks.borrow().clone());
                    }
                }
                ListResult::Error => {
                    error!("Error listing PulseAudio sinks");
                    if let Some(cb) = callback_clone.borrow_mut().take() {
                        cb(Vec::new());
                    }
                }
            });
        });
    }

    pub fn get_sources(self: &Rc<Self>, callback: impl FnOnce(Vec<AudioSource>) + 'static) {
        let mgr_weak = Rc::downgrade(self);
        let ctx = self.context.borrow();
        let intro = ctx.introspect();
        let callback = Rc::new(RefCell::new(Some(callback)));

        // First get server info for default source name
        intro.get_server_info(move |info| {
            let mgr = match mgr_weak.upgrade() {
                Some(m) => m,
                None => {
                    if let Some(cb) = callback.borrow_mut().take() {
                        cb(Vec::new());
                    }
                    return;
                }
            };

            let default_name = info
                .default_source_name
                .as_ref()
                .map(|n| n.to_string())
                .unwrap_or_default();
            *mgr.default_source_name.borrow_mut() = Some(default_name.clone());

            let sources: Rc<RefCell<Vec<AudioSource>>> = Rc::new(RefCell::new(Vec::new()));
            let sources_clone = Rc::clone(&sources);
            let callback_clone = Rc::clone(&callback);

            let ctx2 = mgr.context.borrow();
            let intro2 = ctx2.introspect();
            intro2.get_source_info_list(move |res| match res {
                ListResult::Item(source) => {
                    let name = source
                        .name
                        .as_ref()
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    // Skip monitor sources (they echo output back as input)
                    if name.ends_with(".monitor") {
                        return;
                    }
                    let avg_vol = source.volume.avg();
                    let percent =
                        ((avg_vol.0 as f64 / Volume::NORMAL.0 as f64) * 100.0).min(150.0);
                    sources_clone.borrow_mut().push(AudioSource {
                        index: source.index,
                        name: name.clone(),
                        description: source
                            .description
                            .as_ref()
                            .map(|d| d.to_string())
                            .unwrap_or_else(|| name),
                        volume_percent: percent,
                        muted: source.mute,
                        is_default: source
                            .name
                            .as_ref()
                            .is_some_and(|n| n.to_string() == default_name),
                    });
                }
                ListResult::End => {
                    if let Some(cb) = callback_clone.borrow_mut().take() {
                        cb(sources.borrow().clone());
                    }
                }
                ListResult::Error => {
                    error!("Error listing PulseAudio sources");
                    if let Some(cb) = callback_clone.borrow_mut().take() {
                        cb(Vec::new());
                    }
                }
            });
        });
    }

    pub fn get_apps(self: &Rc<Self>, callback: impl FnOnce(Vec<AudioApp>) + 'static) {
        let ctx = self.context.borrow();
        let intro = ctx.introspect();

        let apps: Rc<RefCell<Vec<AudioApp>>> = Rc::new(RefCell::new(Vec::new()));
        let apps_clone = Rc::clone(&apps);
        let callback_rc = Rc::new(RefCell::new(Some(callback)));
        let callback_clone = Rc::clone(&callback_rc);

        intro.get_sink_input_info_list(move |res| match res {
            ListResult::Item(input) => {
                let name = input
                    .proplist
                    .get_str("application.name")
                    .unwrap_or_else(|| "Unknown".to_string());
                let avg_vol = input.volume.avg();
                let percent = ((avg_vol.0 as f64 / Volume::NORMAL.0 as f64) * 100.0).min(150.0);
                apps_clone.borrow_mut().push(AudioApp {
                    index: input.index,
                    name,
                    sink_index: input.sink,
                    volume_percent: percent,
                    muted: input.mute,
                });
            }
            ListResult::End => {
                if let Some(cb) = callback_clone.borrow_mut().take() {
                    cb(apps.borrow().clone());
                }
            }
            ListResult::Error => {
                error!("Error listing PulseAudio sink inputs");
                if let Some(cb) = callback_clone.borrow_mut().take() {
                    cb(Vec::new());
                }
            }
        });
    }

    pub fn set_sink_volume(self: &Rc<Self>, index: u32, percent: f64) {
        let percent = percent.clamp(0.0, 150.0);
        let mgr_weak = Rc::downgrade(self);
        let ctx = self.context.borrow();
        let intro = ctx.introspect();

        intro.get_sink_info_by_index(index, move |res| {
            if let ListResult::Item(sink) = res {
                if let Some(mgr) = mgr_weak.upgrade() {
                    let vol_val = ((percent / 100.0) * Volume::NORMAL.0 as f64).round() as u32;
                    let mut new_vol = sink.volume;
                    new_vol.set(sink.channel_map.len(), Volume(vol_val));
                    let ctx2 = mgr.context.borrow();
                    let mut intro2 = ctx2.introspect();
                    intro2.set_sink_volume_by_index(index, &new_vol, None);
                }
            }
        });
    }

    pub fn set_source_volume(self: &Rc<Self>, index: u32, percent: f64) {
        let percent = percent.clamp(0.0, 150.0);
        let mgr_weak = Rc::downgrade(self);
        let ctx = self.context.borrow();
        let intro = ctx.introspect();

        intro.get_source_info_by_index(index, move |res| {
            if let ListResult::Item(source) = res {
                if let Some(mgr) = mgr_weak.upgrade() {
                    let vol_val = ((percent / 100.0) * Volume::NORMAL.0 as f64).round() as u32;
                    let mut new_vol = source.volume;
                    new_vol.set(source.channel_map.len(), Volume(vol_val));
                    let ctx2 = mgr.context.borrow();
                    let mut intro2 = ctx2.introspect();
                    intro2.set_source_volume_by_index(index, &new_vol, None);
                }
            }
        });
    }

    pub fn set_app_volume(self: &Rc<Self>, index: u32, percent: f64) {
        let percent = percent.clamp(0.0, 150.0);
        let mgr_weak = Rc::downgrade(self);
        let ctx = self.context.borrow();
        let intro = ctx.introspect();

        intro.get_sink_input_info(index, move |res| {
            if let ListResult::Item(input) = res {
                if let Some(mgr) = mgr_weak.upgrade() {
                    let vol_val = ((percent / 100.0) * Volume::NORMAL.0 as f64).round() as u32;
                    let mut new_vol = input.volume;
                    new_vol.set(input.channel_map.len(), Volume(vol_val));
                    let ctx2 = mgr.context.borrow();
                    let mut intro2 = ctx2.introspect();
                    intro2.set_sink_input_volume(index, &new_vol, None);
                }
            }
        });
    }

    pub fn set_sink_mute(self: &Rc<Self>, index: u32, muted: bool) {
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();
        intro.set_sink_mute_by_index(index, muted, None);
    }

    pub fn set_source_mute(self: &Rc<Self>, index: u32, muted: bool) {
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();
        intro.set_source_mute_by_index(index, muted, None);
    }

    pub fn set_app_mute(self: &Rc<Self>, index: u32, muted: bool) {
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();
        intro.set_sink_input_mute(index, muted, None);
    }

    pub fn set_default_sink(self: &Rc<Self>, name: &str) {
        let mut ctx = self.context.borrow_mut();
        ctx.set_default_sink(name, |success| {
            if !success {
                error!("Failed to set default sink");
            }
        });
    }

    pub fn set_default_source(self: &Rc<Self>, name: &str) {
        let mut ctx = self.context.borrow_mut();
        ctx.set_default_source(name, |success| {
            if !success {
                error!("Failed to set default source");
            }
        });
    }

    #[allow(dead_code)]
    pub fn move_app_to_sink(self: &Rc<Self>, app_index: u32, sink_index: u32) {
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();
        intro.move_sink_input_by_index(app_index, sink_index, None);
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        if let Ok(mut ctx) = self.context.try_borrow_mut() {
            ctx.disconnect();
        }
    }
}
