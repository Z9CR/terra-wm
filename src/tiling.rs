//! Tiling layout engine (feature 2: Dynamic-Tiling).
//!
//! Infinity foundation: all positions are computed in *layer coordinates*
//! on an unbounded row starting at x = 0. The viewport (which part of the
//! layer the monitor shows) is a separate concern: see `view_offset` on
//! [`crate::state::TerraWm`], which is the output mapping position. Moving
//! the viewport later translates the view without touching window positions,
//! so windows keep their relative positions (goal.md detail).
//!
//! Width policy: each piece is `clamp(viewport_width / count, client_min, smallest_monitor_width)`.
//! When many windows hit their minimum width the row overflows the viewport;
//! the overflow is revealed by translating the viewport (infinity-layer).
//!
//! `Direction` is an enum so vertical/grid layouts can be added later without
//! rewriting the engine. The pure math lives in free functions so it can be
//! unit-tested without a wayland display.

use smithay::{
    desktop::{Space, Window},
    output::Output,
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{IsAlive, Logical, Point, Rectangle, Size},
    wayland::{compositor, shell::xdg::SurfaceCachedState},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    #[allow(dead_code)]
    Vertical,
    #[allow(dead_code)]
    Grid,
}

struct Slot {
    window: Window,
    width: i32,
}

pub struct TilingLayout {
    windows: Vec<Slot>,
    focused: Option<usize>,
    direction: Direction,
}

impl Default for TilingLayout {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            direction: Direction::Horizontal,
        }
    }
}

impl TilingLayout {
    pub fn add(&mut self, space: &mut Space<Window>, output: &Output, window: Window) {
        let idx = match self.focused {
            Some(f) => f + 1,
            None => self.windows.len(),
        };
        self.windows.insert(idx, Slot { window, width: 0 });
        self.focused = Some(idx);
        self.rebalance(output);
        self.relayout(space, output);
    }

    pub fn set_focus(&mut self, space: &mut Space<Window>, output: &Output, window: &Window) {
        if let Some(idx) = self.windows.iter().position(|slot| &slot.window == window) {
            self.focused = Some(idx);
            self.relayout(space, output);
        }
    }

    pub fn clear_focus(&mut self, space: &mut Space<Window>, output: &Output) {
        if self.focused.take().is_some() {
            self.relayout(space, output);
        }
    }

    /// The currently focused window, if any (used by future keyboard commands).
    #[allow(dead_code)]
    pub fn focused_window(&self) -> Option<Window> {
        self.focused.map(|idx| self.windows[idx].window.clone())
    }

    pub fn width_of(&self, window: &Window) -> Option<i32> {
        self.windows
            .iter()
            .find(|slot| &slot.window == window)
            .map(|slot| slot.width)
    }

    /// Resize a window to an absolute target width; the adjacent window
    /// compensates. `left_edge` selects which neighbor compensates and which
    /// way the delta moves the border.
    pub fn resize_window(
        &mut self,
        space: &mut Space<Window>,
        output: &Output,
        window: &Window,
        new_width: i32,
        left_edge: bool,
    ) {
        let Some(idx) = self.windows.iter().position(|slot| &slot.window == window) else {
            return;
        };
        let n = self.windows.len();
        let neighbor = if left_edge {
            idx.checked_sub(1)
        } else {
            (idx + 1 < n).then_some(idx + 1)
        };
        let Some(neighbor) = neighbor else {
            return;
        };

        let min_i = min_width_of(&self.windows[idx].window);
        let min_neighbor = min_width_of(&self.windows[neighbor].window);

        let new_width = new_width.max(min_i);
        let adjusted = new_width - self.windows[idx].width;
        self.windows[idx].width = new_width;
        self.windows[neighbor].width = (self.windows[neighbor].width - adjusted).max(min_neighbor);

        self.relayout(space, output);
    }

    pub fn cleanup(&mut self, space: &mut Space<Window>, output: &Output) {
        let mut changed = false;
        self.windows.retain(|slot| {
            if slot.window.alive() {
                true
            } else {
                changed = true;
                false
            }
        });

        if !changed {
            return;
        }

        if self.focused.is_some_and(|f| f >= self.windows.len()) {
            self.focused = self.windows.len().checked_sub(1);
        }
        self.rebalance(output);
        self.relayout(space, output);
    }

    fn rebalance(&mut self, output: &Output) {
        let count = self.windows.len() as i32;
        if count == 0 {
            return;
        }
        let viewport_w = viewport_width(output);
        let share = (viewport_w / count).max(1);
        for slot in &mut self.windows {
            slot.width = equal_share(share, min_width_of(&slot.window), viewport_w);
        }
    }

    fn relayout(&mut self, space: &mut Space<Window>, output: &Output) {
        let viewport_h = viewport_height(output);
        match self.direction {
            Direction::Horizontal => {
                let widths: Vec<i32> = self.windows.iter().map(|slot| slot.width).collect();
                let rects = row_positions(&widths, viewport_h);
                for (idx, (slot, rect)) in self.windows.iter().zip(rects).enumerate() {
                    space.map_element(slot.window.clone(), rect.loc, Some(idx) == self.focused);

                    let toplevel = slot.window.toplevel().unwrap();
                    toplevel.with_pending_state(|state| {
                        state.size = Some(rect.size);
                        state.states.set(xdg_toplevel::State::Activated);
                    });
                    toplevel.send_pending_configure();
                }
            }
            Direction::Vertical | Direction::Grid => {
                unimplemented!("vertical and grid directions land with feature 3")
            }
        }
    }
}

pub fn equal_share(share: i32, min_w: i32, cap_w: i32) -> i32 {
    share.clamp(min_w, cap_w)
}

pub fn row_positions(widths: &[i32], viewport_h: i32) -> Vec<Rectangle<i32, Logical>> {
    let mut x = 0;
    widths
        .iter()
        .map(|w| {
            let rect = Rectangle::new(Point::from((x, 0)), Size::from((*w, viewport_h)));
            x += w;
            rect
        })
        .collect()
}

fn min_width_of(window: &Window) -> i32 {
    window
        .toplevel()
        .map(|toplevel| {
            compositor::with_states(toplevel.wl_surface(), |states| {
                states
                    .cached_state
                    .get::<SurfaceCachedState>()
                    .current()
                    .min_size
                    .w
                    .max(1)
            })
        })
        .unwrap_or(1)
}

fn viewport_size(output: &Output) -> Size<i32, Logical> {
    output
        .current_mode()
        .map(|mode| {
            mode.size
                .to_f64()
                .to_logical(output.current_scale().fractional_scale())
                .to_i32_round()
        })
        .map(|size| output.current_transform().transform_size(size))
        .unwrap_or_default()
}

fn viewport_width(output: &Output) -> i32 {
    viewport_size(output).w
}

fn viewport_height(output: &Output) -> i32 {
    viewport_size(output).h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_share_halves() {
        assert_eq!(equal_share(960, 1, 1920), 960);
        assert_eq!(equal_share(640, 1, 1920), 640);
    }

    #[test]
    fn equal_share_capped_by_smallest_monitor() {
        // one window on a 3840 viewport: capped to smallest monitor width
        assert_eq!(equal_share(3840, 1, 1920), 1920);
    }

    #[test]
    fn equal_share_floored_by_client_min() {
        assert_eq!(equal_share(100, 320, 1920), 320);
    }

    #[test]
    fn row_positions_are_cumulative() {
        let rects = row_positions(&[100, 200, 50], 1080);
        assert_eq!(rects[0].loc, Point::from((0, 0)));
        assert_eq!(rects[1].loc, Point::from((100, 0)));
        assert_eq!(rects[2].loc, Point::from((300, 0)));
        assert_eq!(rects[2].size, Size::from((50, 1080)));
    }

    #[test]
    fn row_positions_overflow_past_viewport() {
        // total width exceeds any single viewport: positions keep growing,
        // the extra is revealed by translating the viewport (infinity-layer)
        let rects = row_positions(&[800, 800, 800], 1080);
        assert_eq!(rects[2].loc.x, 1600);
    }
}
