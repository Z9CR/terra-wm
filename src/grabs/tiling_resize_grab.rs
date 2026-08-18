use smithay::{
    desktop::Window,
    input::pointer::{
        AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
        GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent,
        GestureSwipeEndEvent, GestureSwipeUpdateEvent, GrabStartData as PointerGrabStartData,
        MotionEvent, PointerGrab, PointerInnerHandle, RelativeMotionEvent,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
};

use crate::state::TerraWm;

pub struct TilingResizeGrab {
    pub start_data: PointerGrabStartData<TerraWm>,
    pub window: Window,
}

impl PointerGrab<TerraWm> for TilingResizeGrab {
    fn motion(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let delta = (event.location.x - self.start_data.location.x) as i32;
        data.tiling
            .resize_delta(&mut data.space, &data.output, &self.window, delta);
    }

    fn relative_motion(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);

        const BTN_LEFT: u32 = 0x110;

        if !handle.current_pressed().contains(&BTN_LEFT) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(&mut self, data: &mut TerraWm, handle: &mut PointerInnerHandle<'_, TerraWm>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event)
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event)
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event)
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event)
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event)
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event)
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event)
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut TerraWm,
        handle: &mut PointerInnerHandle<'_, TerraWm>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event)
    }

    fn start_data(&self) -> &PointerGrabStartData<TerraWm> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut TerraWm) {}
}
