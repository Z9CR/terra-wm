use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent},
    },
    output::Mode,
    reexports::{calloop::EventLoop, winit::window::WindowAttributes},
    utils::Transform,
};

use crate::{render::render_frame, state::TerraWm};

pub fn init_winit(
    event_loop: &mut EventLoop<TerraWm>,
    state: &mut TerraWm,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init_from_attributes::<GlesRenderer>(
        WindowAttributes::default().with_title("terra-wm"),
    )?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let _global = state.output.create_global::<TerraWm>(&state.display_handle);
    state.output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    state.output.set_preferred(mode);

    for layer in &mut state.layer_stack {
        layer.space.map_output(&state.output, state.view_offset);
    }

    let mut damage_tracker = OutputDamageTracker::from_output(&state.output);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| {
            match event {
                WinitEvent::Resized { size, .. } => {
                    state.output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => {
                    render_frame(&mut backend, state, &mut damage_tracker);
                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => state.loop_signal.stop(),
                _ => (),
            };
        })?;

    Ok(())
}
