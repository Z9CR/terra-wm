mod grabs;
mod handlers;
mod input;
mod render;
mod state;
mod tiling;
mod winit;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::TerraWm;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let mut event_loop = EventLoop::try_new()?;
    let display = Display::new()?;
    let mut state = TerraWm::new(&mut event_loop, display);

    crate::winit::init_winit(&mut event_loop, &mut state)?;
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket_name) };

    spawn_client();
    event_loop.run(None, &mut state, |_| {})?;

    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}

fn spawn_client() {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("-c") | Some("--command"), Some(command)) => {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c").arg(command);
            if let Err(e) = cmd.spawn() {
                tracing::warn!(error = %e, "failed to spawn client");
            }
        }
        _ => (),
    }
}
