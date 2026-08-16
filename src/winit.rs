use smithay::{
    backend::{
        renderer::gles::GlesRenderer,
        winit::{self, WinitEvent},
    },
    reexports::{calloop::EventLoop, winit::window::WindowAttributes},
};

use crate::{render::render_frame, state::TerraWm};

pub fn init_winit(
    event_loop: &mut EventLoop<TerraWm>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init_from_attributes::<GlesRenderer>(
        WindowAttributes::default().with_title("terra-wm"),
    )?;

    event_loop.handle().insert_source(winit, move |event, _, state| {
        match event {
            WinitEvent::Resized { .. } => (),
            WinitEvent::Redraw => {
                render_frame(&mut backend, state);
                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => (),
        };
    })?;

    Ok(())
}
