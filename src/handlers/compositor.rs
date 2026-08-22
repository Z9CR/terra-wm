use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    desktop::{Space, Window, WindowSurfaceType, layer_map_for_output},
    reexports::wayland_server::{
        Client,
        protocol::{wl_buffer, wl_surface::WlSurface},
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState, get_parent,
            is_sync_subsurface,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::{
    grabs::resize_grab,
    state::{ClientState, TerraWm},
};

use super::xdg_shell;

impl CompositorHandler for TerraWm {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            for layer in &mut self.layer_stack {
                if let Some(window) = layer
                    .space
                    .elements()
                    .find(|w| w.toplevel().unwrap().wl_surface() == &root)
                {
                    window.on_commit();
                    break;
                }
            }
        };

        xdg_shell::handle_commit(&mut self.popups, &self.layer_stack[0].space, surface);
        for layer in &mut self.layer_stack {
            resize_grab::handle_commit(&mut layer.space, surface);
        }
        handle_layer_commit(&mut self.layer_stack[0].space, surface);
    }
}

fn handle_layer_commit(space: &Space<Window>, surface: &WlSurface) {
    if let Some(output) = space.outputs().find(|o| {
        layer_map_for_output(o)
            .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
            .is_some()
    }) {
        let mut map = layer_map_for_output(output);
        map.arrange();
        if let Some(layer) = map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL) {
            layer.layer_surface().send_pending_configure();
        }
    }
}

impl BufferHandler for TerraWm {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for TerraWm {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
