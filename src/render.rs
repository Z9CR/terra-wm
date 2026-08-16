use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::WinitGraphicsBackend,
    },
    desktop::space::render_output,
    utils::Rectangle,
};

use crate::state::TerraWm;

pub fn render_frame(
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    state: &mut TerraWm,
    damage_tracker: &mut OutputDamageTracker,
) {
    let size = backend.window_size();
    let damage = Rectangle::from_size(size);

    {
        let (renderer, mut framebuffer) = backend.bind().unwrap();
        render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
            &state.output,
            renderer,
            &mut framebuffer,
            1.0,
            0,
            [&state.space],
            &[],
            damage_tracker,
            [0.1, 0.1, 0.1, 1.0],
        )
        .unwrap();
    }

    state.space.elements().for_each(|window| {
        window.send_frame(
            &state.output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(state.output.clone()),
        )
    });

    state.space.refresh();
    state.popups.cleanup();
    let _ = state.display_handle.flush_clients();

    backend.submit(Some(&[damage])).unwrap();
}
