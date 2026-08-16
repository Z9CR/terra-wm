use smithay::{
    backend::{
        renderer::{
            Color32F, Frame, Renderer,
            element::{
                Kind,
                surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
            },
            gles::GlesRenderer,
            utils::draw_render_elements,
        },
        winit::WinitGraphicsBackend,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Rectangle, Transform},
    wayland::compositor::{SurfaceAttributes, TraversalAction, with_surface_tree_downward},
};

use crate::state::TerraWm;

pub fn render_frame(backend: &mut WinitGraphicsBackend<GlesRenderer>, state: &mut TerraWm) {
    let size = backend.window_size();
    let damage = Rectangle::from_size(size);

    {
        let (renderer, mut framebuffer) = backend.bind().unwrap();
        let elements = state
            .xdg_shell_state
            .toplevel_surfaces()
            .iter()
            .flat_map(|surface| {
                render_elements_from_surface_tree(
                    renderer,
                    surface.wl_surface(),
                    (0, 0),
                    1.0,
                    1.0,
                    Kind::Unspecified,
                )
            })
            .collect::<Vec<WaylandSurfaceRenderElement<GlesRenderer>>>();

        let mut frame = renderer
            .render(&mut framebuffer, size, Transform::Flipped180)
            .unwrap();
        frame.clear(Color32F::new(0.1, 0.1, 0.1, 1.0), &[damage]).unwrap();
        draw_render_elements(&mut frame, 1.0, &elements, &[damage]).unwrap();
        let _ = frame.finish().unwrap();
    }

    for surface in state.xdg_shell_state.toplevel_surfaces() {
        send_frames_surface_tree(
            surface.wl_surface(),
            state.start_time.elapsed().as_millis() as u32,
        );
    }

    let _ = state.display_handle.flush_clients();

    backend.submit(Some(&[damage])).unwrap();
}

pub fn send_frames_surface_tree(surface: &WlSurface, time: u32) {
    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}
