//! vscreen: a layer is divided into rectangles of the smallest physical
//! monitor size (goal.md proper nouns). Used to place maximized windows,
//! and later to navigate the infinity-layer.

use smithay::{
    output::Output,
    utils::{Logical, Point, Rectangle, Size},
};

/// Grid index of the vscreen containing a point (layer coordinates).
///
/// Border tie-break per goal.md: a point exactly on a vscreen border
/// belongs to the right-top vscreen. For a vertical border the floor
/// division already lands on the right vscreen; for a horizontal border
/// the top vscreen is one row above.
pub fn vscreen_of(point: Point<i32, Logical>, cell: Size<i32, Logical>) -> Point<i32, Logical> {
    let x = point.x.div_euclid(cell.w);
    let mut y = point.y.div_euclid(cell.h);
    if point.y > 0 && point.y.rem_euclid(cell.h) == 0 {
        y -= 1;
    }
    Point::from((x, y))
}

/// The rectangle of the vscreen at a grid index (layer coordinates).
pub fn vscreen_rect(
    index: Point<i32, Logical>,
    cell: Size<i32, Logical>,
) -> Rectangle<i32, Logical> {
    Rectangle::new(Point::from((index.x * cell.w, index.y * cell.h)), cell)
}

/// The size of the smallest physical monitor: element-wise minimum of the
/// logical sizes of all outputs.
pub fn smallest_monitor_size<'a>(
    outputs: impl Iterator<Item = &'a Output>,
) -> Option<Size<i32, Logical>> {
    outputs
        .filter_map(|output| {
            output.current_mode().map(|mode| {
                let size: Size<i32, Logical> = mode
                    .size
                    .to_f64()
                    .to_logical(output.current_scale().fractional_scale())
                    .to_i32_round();
                output.current_transform().transform_size(size)
            })
        })
        .reduce(|a, b| Size::from((a.w.min(b.w), a.h.min(b.h))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell() -> Size<i32, Logical> {
        Size::from((100, 80))
    }

    #[test]
    fn center_inside_vscreen() {
        assert_eq!(
            vscreen_of(Point::from((50, 40)), cell()),
            Point::from((0, 0))
        );
        assert_eq!(
            vscreen_of(Point::from((250, 100)), cell()),
            Point::from((2, 1))
        );
    }

    #[test]
    fn vertical_border_goes_right() {
        // exactly on the border between column 0 and 1 -> right (column 1)
        assert_eq!(
            vscreen_of(Point::from((100, 40)), cell()),
            Point::from((1, 0))
        );
    }

    #[test]
    fn horizontal_border_goes_top() {
        // exactly on the border between row 0 and 1 -> top (row 0)
        assert_eq!(
            vscreen_of(Point::from((50, 80)), cell()),
            Point::from((0, 0))
        );
    }

    #[test]
    fn corner_goes_right_top() {
        assert_eq!(
            vscreen_of(Point::from((100, 80)), cell()),
            Point::from((1, 0))
        );
    }

    #[test]
    fn rect_from_index() {
        let rect = vscreen_rect(Point::from((1, 2)), cell());
        assert_eq!(rect.loc, Point::from((100, 160)));
        assert_eq!(rect.size, cell());
    }
}
