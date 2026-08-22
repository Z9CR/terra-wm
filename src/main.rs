mod grabs;
mod handlers;
mod input;
mod layer;
mod render;
mod state;
mod udev;
mod vscreen;
mod winit;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::TerraWm;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let mut args = std::env::args().skip(1);
    let backend_arg = args.next();
    let backend = backend_arg.as_deref();
    let mut command = None;
    while let Some(arg) = args.next() {
        if (arg == "-c" || arg == "--command") && command.is_none() {
            command = args.next();
        }
    }

    match backend {
        Some("--winit") => run_winit(command),
        Some("--tty-udev") => run_udev(command),
        _ => {
            println!("usage: terra-wm [--winit|--tty-udev] [-c command]");
            Ok(())
        }
    }
}

fn run_winit(command: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop = EventLoop::try_new()?;
    let display = Display::new()?;
    let mut state = TerraWm::new(&mut event_loop, display);

    crate::winit::init_winit(&mut event_loop, &mut state)?;
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket_name) };

    spawn_client(command);
    event_loop.run(None, &mut state, |_| {})?;

    Ok(())
}

fn run_udev(command: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // udev.rs sets WAYLAND_DISPLAY itself before spawning clients
    crate::udev::run_udev(command)
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}

fn spawn_client(command: Option<String>) {
    if let Some(command) = command {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        if let Err(e) = cmd.spawn() {
            tracing::warn!(error = %e, "failed to spawn client");
        }
    }
}
