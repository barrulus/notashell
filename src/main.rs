mod app;
mod config;
mod controls;
mod daemon;
mod dbus;
mod ui;

use clap::Parser;
use gtk4::Application;
use gtk4::glib;
use gtk4::prelude::*;

/// A floating system control panel for Wayland compositors (Hyprland/Sway)
#[derive(Parser, Debug)]
#[command(name = "notashell", version, about)]
struct Args {
    /// Toggle the panel visibility (sends signal to running daemon)
    #[arg(long)]
    toggle: bool,

    /// Reload config and CSS (sends signal to running daemon)
    #[arg(long)]
    reload: bool,

    /// Toggle expanded/compact panel size (sends signal to running daemon)
    #[arg(long)]
    resize: bool,

    /// Run in foreground instead of daemonizing
    #[arg(long, short)]
    foreground: bool,

    /// Internal: actual daemon process (not for end users)
    #[arg(long, hide = true)]
    daemon: bool,
}

const APP_ID: &str = "com.github.notashell.Notashell";

fn main() {
    // Initialize logging
    env_logger::init();

    let args = Args::parse();

    if args.toggle {
        // Send Toggle() to running daemon and exit
        let rt = glib::MainContext::default();
        rt.block_on(async {
            if daemon::is_instance_running().await {
                match daemon::send_toggle().await {
                    Ok(_) => log::info!("Toggle sent to running instance"),
                    Err(e) => {
                        log::error!("Failed to send toggle: {e}");
                        eprintln!("Error: could not toggle — is notashell running?");
                    }
                }
            } else {
                eprintln!("No running instance found. Start with: notashell");
            }
        });
        return;
    }

    if args.reload {
        // Send Reload() to running daemon and exit
        let rt = glib::MainContext::default();
        rt.block_on(async {
            if daemon::is_instance_running().await {
                match daemon::send_reload().await {
                    Ok(_) => {
                        log::info!("Reload sent to running instance");
                        println!("Config and CSS reloaded");
                    }
                    Err(e) => {
                        log::error!("Failed to send reload: {e}");
                        eprintln!("Error: could not reload — is notashell running?");
                    }
                }
            } else {
                eprintln!("No running instance found. Start with: notashell");
            }
        });
        return;
    }

    if args.resize {
        // Send Resize() to running daemon and exit
        let rt = glib::MainContext::default();
        rt.block_on(async {
            if daemon::is_instance_running().await {
                match daemon::send_resize().await {
                    Ok(_) => log::info!("Resize sent to running instance"),
                    Err(e) => {
                        log::error!("Failed to send resize: {e}");
                        eprintln!("Error: could not resize — is notashell running?");
                    }
                }
            } else {
                eprintln!("No running instance found. Start with: notashell");
            }
        });
        return;
    }

    // Daemonize: re-exec as a background process with detached stdio
    if !args.foreground && !args.daemon {
        let exe = std::env::current_exe().unwrap_or_else(|e| {
            eprintln!("Failed to determine executable path: {e}");
            std::process::exit(1);
        });

        // Open a log file for daemon stderr so env_logger output is captured
        let log_file = std::fs::File::create("/tmp/notashell.log")
            .map(std::process::Stdio::from)
            .unwrap_or_else(|_| std::process::Stdio::null());

        match std::process::Command::new(exe)
            .arg("--daemon")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(log_file)
            .spawn()
        {
            Ok(child) => {
                eprintln!(
                    "notashell daemon started (pid {}). Use --toggle to show/hide.",
                    child.id()
                );
            }
            Err(e) => {
                eprintln!("Failed to start daemon: {e}");
                std::process::exit(1);
            }
        }

        return;
    }

    // Start the GTK application (daemon mode)
    log::info!("Starting notashell daemon");

    let app = Application::builder().application_id(APP_ID).build();

    // Catch kill signals to cleanly shut down GTK and drop hardware locks
    const SIGINT: i32 = 2;
    const SIGTERM: i32 = 15;

    let app_clone = app.clone();
    glib::unix_signal_add_local(SIGTERM, move || { // SIGTERM
        log::info!("Received SIGTERM, gracefully shutting down");
        app_clone.quit();
        glib::ControlFlow::Break
    });
    
    let app_clone2 = app.clone();
    glib::unix_signal_add_local(SIGINT, move || { // SIGINT
        log::info!("Received SIGINT, gracefully shutting down");
        app_clone2.quit();
        glib::ControlFlow::Break
    });

    app.connect_activate(|app| {
        log::info!("Application activated");

        // Build the UI (starts hidden)
        let widgets = ui::window::build_window(app);

        // Create a send-safe weak reference for cross-thread window access
        let window_ref = {
            use gtk4::glib::object::ObjectExt;
            widgets.window.downgrade().into() // SendWeakRef
        };
        let window_ref: glib::SendWeakRef<gtk4::ApplicationWindow> = window_ref;

        // Create panel state with visibility toggle callback
        // This callback is called from the D-Bus thread, so it dispatches
        // to the GTK main thread via MainContext::invoke (thread-safe).
        let panel_state = daemon::PanelState::new(move |visible| {
            let window_ref = window_ref.clone();
            glib::MainContext::default().invoke(move || {
                if let Some(window) = window_ref.upgrade() {
                    if visible {
                        window.present();
                    } else {
                        window.set_visible(false);
                    }
                }
            });
        });

        // Register the D-Bus daemon service
        let panel_state_clone = panel_state.clone();
        glib::spawn_future_local(async move {
            match daemon::register_service(panel_state_clone).await {
                Ok(_conn) => {
                    log::info!("Daemon D-Bus service ready");
                    // _conn is kept alive by the async task
                    // It will be dropped when the app exits
                    std::future::pending::<()>().await;
                }
                Err(e) => {
                    log::error!("Failed to register D-Bus service: {e}");
                }
            }
        });

        // Connect to NetworkManager and set up the app controller
        let panel_state_for_app = panel_state.clone();
        glib::spawn_future_local(async move {
            match dbus::network_manager::ConnectionManager::new().await {
                Ok(wifi) => {
                    log::info!("NetworkManager D-Bus connection established");
                    let config = config::Config::load();
                    app::setup(
                        &widgets,
                        wifi,
                        panel_state_for_app.scan_requested.clone(),
                        panel_state_for_app.clone(),
                    );

                    // Only show panel on start if configured
                    if config.show_on_start {
                        panel_state_for_app.show();
                    }
                }
                Err(e) => {
                    log::error!("Failed to connect to NetworkManager: {e}");
                    widgets
                        .status_label
                        .set_text("Error: NetworkManager unavailable");
                    // Still show the panel so user sees the error
                    panel_state_for_app.show();
                }
            }
        });
    });

    // Pass no CLI args to GTK — our own args (--toggle, --daemon, etc.)
    // are already consumed by clap and would confuse GApplication.
    app.run_with_args::<String>(&[]);
    
    // Allow pending D-Bus responses and GTK callbacks to complete before process exit.
    // Iterating the main context processes the teardown events gracefully.
    let ctx = glib::MainContext::default();
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(150);
    
    while ctx.pending() && start.elapsed() < timeout {
        ctx.iteration(false);
    }
}
