//! Layer abstraction (feature 2: multi-layers overlay).
//!
//! A layer is an infinite 2D plane holding its own `Space<Window>`. Windows
//! are stacked on it labwc-style (free positions, raise-to-top ordering).
//! Layers live in a stack rendered bottom-up; monitors are viewports into
//! the layers (see `view_offset`). For infinity-layer, each layer is divided
//! into vscreens of smallest-monitor size (goal.md proper nouns).
//!
//! `layer_stack[0]` is the bottom layer, the last element the top layer.
//! Rendering passes the spaces in reverse order so the bottom layer is
//! drawn first (see `render.rs`).

use smithay::{
    desktop::{Space, Window, WindowSurfaceType},
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
};

pub struct Layer {
    pub space: Space<Window>,
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            space: Space::default(),
        }
    }
}

impl Layer {
    /// Place a new window labwc-style: cascade from the top-left corner,
    /// 24px per window, restarting the cascade every 10 windows.
    pub fn add_window(&mut self, window: Window) {
        let offset = 24 * (self.space.elements().count() as i32 % 10);
        self.space.map_element(window, (offset, offset), true);
    }

    pub fn focus_window(&mut self, window: &Window) {
        self.space.raise_element(window, true);
    }

    pub fn clear_focus(&mut self) {
        self.space.elements().for_each(|window| {
            window.set_activated(false);
        });
    }

    pub fn cleanup(&mut self) {
        self.space.refresh();
    }

    pub fn element_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(&Window, Point<i32, Logical>)> {
        self.space.element_under(pos)
    }

    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_constructs_empty() {
        let layer = Layer::default();
        assert_eq!(layer.space.elements().count(), 0);
    }
}
