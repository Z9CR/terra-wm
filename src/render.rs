use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::WinitGraphicsBackend,
    },
    desktop::{layer_map_for_output, space::render_output},
    output::Output,
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
        // Spaces are drawn in reverse order, so pass the layer stack reversed:
        // layer_stack[0] (bottom) is drawn first, the top layer last.
        let spaces = state.layer_stack.iter().rev().map(|layer| &layer.space);
        render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
            &state.output,
            renderer,
            &mut framebuffer,
            1.0,
            0,
            spaces,
            &[],
            damage_tracker,
            [0.1, 0.1, 0.1, 1.0],
        )
        .unwrap();
    }

    let output = state.output.clone();
    post_render(state, &output);

    backend.submit(Some(&[damage])).unwrap();
}

/// Shared post-render work for both backends: frame callbacks, cleanup,
/// client flush.
pub fn post_render(state: &mut TerraWm, output: &Output) {
    for layer in &state.layer_stack {
        layer.space.elements().for_each(|window| {
            window.send_frame(
                output,
                state.start_time.elapsed(),
                Some(Duration::ZERO),
                |_, _| Some(output.clone()),
            )
        });
    }

    let map = layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        );
    }

    for layer in &mut state.layer_stack {
        layer.cleanup();
    }
    state.popups.cleanup();
    let _ = state.display_handle.flush_clients();
}
