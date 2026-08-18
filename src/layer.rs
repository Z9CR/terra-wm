//! Layer abstraction (feature 3: multi-layers overlay).
//!
//! A layer is an infinite 2D plane holding its own `Space<Window>` plus a
//! layout strategy selected by the user-editable `window_layout_type`
//! property (goal.md proper nouns). Layers live in a stack rendered
//! bottom-up; monitors are viewports into the layers (see `view_offset`).
//!
//! `layer_stack[0]` is the bottom layer, the last element the top layer.
//! Rendering passes the spaces in reverse order so the bottom layer is
//! drawn first (see `render.rs`).

use smithay::{
    desktop::{Space, Window, WindowSurfaceType},
    output::Output,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
};

use crate::tiling::TilingLayout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutType {
    Tiling,
    Stacked,
}

pub struct Layer {
    pub layout_type: LayoutType,
    pub space: Space<Window>,
    pub tiling: TilingLayout,
}

impl Layer {
    pub fn new_tiling() -> Self {
        Self {
            layout_type: LayoutType::Tiling,
            space: Space::default(),
            tiling: TilingLayout::default(),
        }
    }

    /// A stacked layer keeps windows at free positions with raise-to-top
    /// ordering. Constructor lands with layer creation (feature 4).
    #[allow(dead_code)]
    pub fn new_stacked() -> Self {
        Self {
            layout_type: LayoutType::Stacked,
            space: Space::default(),
            tiling: TilingLayout::default(),
        }
    }

    pub fn add_window(&mut self, output: &Output, window: Window) {
        match self.layout_type {
            LayoutType::Tiling => self.tiling.add(&mut self.space, output, window),
            LayoutType::Stacked => {
                let offset = 24 * (self.space.elements().count() as i32 % 10);
                self.space.map_element(window, (offset, offset), true);
            }
        }
    }

    pub fn focus_window(&mut self, output: &Output, window: &Window) {
        match self.layout_type {
            LayoutType::Tiling => self.tiling.set_focus(&mut self.space, output, window),
            LayoutType::Stacked => {
                self.space.raise_element(window, true);
            }
        }
    }

    pub fn clear_focus(&mut self, output: &Output) {
        match self.layout_type {
            LayoutType::Tiling => self.tiling.clear_focus(&mut self.space, output),
            LayoutType::Stacked => {
                self.space.elements().for_each(|window| {
                    window.set_activated(false);
                });
            }
        }
    }

    pub fn cleanup(&mut self, output: &Output) {
        self.space.refresh();
        match self.layout_type {
            LayoutType::Tiling => self.tiling.cleanup(&mut self.space, output),
            LayoutType::Stacked => {}
        }
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

    pub fn resize_window(
        &mut self,
        output: &Output,
        window: &Window,
        new_width: i32,
        left_edge: bool,
    ) {
        if self.layout_type == LayoutType::Tiling {
            self.tiling
                .resize_window(&mut self.space, output, window, new_width, left_edge);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacked_layer_constructs() {
        let layer = Layer::new_stacked();
        assert_eq!(layer.layout_type, LayoutType::Stacked);
        assert_eq!(layer.space.elements().count(), 0);
    }

    #[test]
    fn tiling_layer_constructs() {
        let layer = Layer::new_tiling();
        assert_eq!(layer.layout_type, LayoutType::Tiling);
    }
}
