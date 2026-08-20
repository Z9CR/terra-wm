# memory pool

## env facts
- sandbox: workspace-write mode, only `terra-wm/` writable
- `/tmp` is per-command tmpfs (NOT persistent between bash calls)
- `/run/user/1000` read-only, so wayland sockets must live in workspace `.runtime/` (XDG_RUNTIME_DIR)
- `~/.cargo` read-only -> must build with `CARGO_HOME=terra-wm/.cargo-home`
- X server on `:0` (1920x1080); no `/dev/dri` in sandbox; Mesa llvmpipe software GL works via EGL
- labwc 0.20.1 installed, headless-capable (WLR_BACKENDS=headless WLR_RENDERER=pixman)
- smithay master source at `../smithay` (anvil + smallvil examples, v0.7.0+428 commits)

## decisions (2026/8/17, milestone 0)
- smithay dep: crates.io 0.7.0 (features: backend_winit, renderer_gl, wayland_frontend)
- backend/renderer: winit + GlesRenderer, host compositor = labwc (Wayland, no X11 involved)
- scope: minimal render (compositor + xdg_shell + shm + render; NO seat/input yet)
- test: self-written wayland-client example `test_client` (colored xdg window, exits after 5 frames)

## test chain recipe
```
XDG_RUNTIME_DIR=terra-wm/.runtime WLR_BACKENDS=headless WLR_RENDERER=pixman \
  WLR_HEADLESS_OUTPUTS=1 WLR_LIBINPUT_NO_DEVICES=1 labwc -C terra-wm/.runtime/labwc -d
XDG_RUNTIME_DIR=terra-wm/.runtime WAYLAND_DISPLAY=wayland-0 WINIT_UNIX_BACKEND=wayland \
  LIBGL_ALWAYS_SOFTWARE=1 terra-wm -c target/debug/examples/test_client
```
verified: client connects, receives configure, 5 frame callbacks, exit 0
verified on host (tty2, 2026/8/17): bare run errors out (expected, winit needs a host);
  DISPLAY=:0 WINIT_UNIX_BACKEND=x11 works, `-c konsole` launches real Qt client inside compositor

## architecture decision (2026/8/17)
- DRM/udev backend DEFERRED: smithay separates backend from compositor logic, our backend
  seam is already clean (only winit.rs / render.rs / main.rs touch backend types)
- rule: future milestones (stacking/tiling/layers/infinity) must never leak backend types
  into state.rs / handlers.rs / desktop logic; when DRM comes, add Backend trait + udev.rs
  anvil-style, window-management code stays untouched. DRM will include libinput input.

## smithay 0.7.0 api notes
- per-protocol macros `delegate_compositor!` `delegate_xdg_shell!` `delegate_shm!` (NOT `delegate_dispatch2!`, that is master-only)
- Rust 2024: delegate macros must be `use`d explicitly (e.g. `use smithay::delegate_seat;`)
- calloop 0.14: `Generic` lives at `calloop::generic::Generic`
- `delegate_xdg_shell!` forces `SeatHandler` impl (XdgPopup dispatch bound); no seat global needed
- winit backend requires GL: `winit::init_from_attributes::<GlesRenderer>(attrs)`
- `WinitEventLoop` is a calloop EventSource; render on `WinitEvent::Redraw`, `request_redraw()` after each frame
- `ShmState::new::<Self>(&dh, vec![])`; `CompositorState::new::<Self>(&dh)`; `XdgShellState::new::<Self>(&dh)`
- `delegate_output!` covers wl_output AND xdg_output; `Output::create_global::<D>(&dh)` registers an output
- `OutputManagerState::new_with_xdg_output` has no Drop side effects: dropping it does NOT remove the global
- `PhysicalProperties` in 0.7.0 has NO `serial_number` field (master only)
- `PointerConstraintsHandler` in 0.7.0 requires `new_constraint` + `cursor_position_hint` (master has defaults)
- `DataDeviceHandler::data_device_state(&self) -> &DataDeviceState` (not &mut)
- DnD in 0.7.0: `start_drag` auto-creates+sets DnDGrab internally; just impl `ClientDndGrabHandler` (started/dropped) + `ServerDndGrabHandler` (send); NO `DndGrabHandler`/`GrabType`/`Source` (master-only)
- `SeatState::new_wl_seat(&dh, name)` creates seat + wl_seat global; `add_keyboard(Default::default(), 200, 25)` / `add_pointer()`
- input processing: generic `process_input_event<I: InputBackend>` (winit today, libinput later with zero changes)
- render: `desktop::space::render_output` with `OutputDamageTracker::from_output`, space mapped output, `Window::send_frame`
- ToplevelSurface has no `title()` accessor in 0.7.0 (title lives in XdgToplevelSurfaceData)
- layer-shell (wlr_layer): `WlrLayerShellState::new::<D>(&dh)`, trait uses PROTOCOL `wlr_layer::LayerSurface`
  (desktop wrapper built via `LayerSurface::new(surface, namespace)`); `delegate_layer_shell!`
- LayerMap: `map_layer`/`unmap_layer`/`arrange`; arrange only sends configure AFTER initial configure
  (spec: initial configure must follow first commit); commit handler: arrange + `send_pending_configure()`
  (returns Some while `!initial_configure_sent`)
- layer rendering is automatic: `space_render_elements` z-orders Background/Bottom below windows,
  Top/Overlay above; just map into LayerMap + send frame callbacks per layer surface
- `-c` spawn: use `sh -c` (Command::new treats whole string as executable, breaks multi-word args)

## milestone: wlr-layer-shell (2026/8/17)
- swaybg -c '#66ccff' now works (was "missing a required Wayland interface": swaybg is a
  layer-shell client, needs zwlr_layer_shell_v1)
- groundwork for goal feat 3 (multi-layers overlay) / 4 (switch two layers)

## milestone: input support (2026/8/17)
- reached smallvil parity: seat(kbd+ptr), winit input, Space+Output+xdg_output, move/resize grabs, popups, data device+dnd
- verified nested under labwc headless: konsole opens real window ("new toplevel"), test client 5 frames, 0 errors
- human.md new rule: run `cargo fmt` after coding

## pitfalls
- `cargo build --example` does NOT rebuild the main bin (stale `Hello, world!` binary bit us)
- `pkill -f "target/debug/terra-wm"` kills the calling shell too (pattern matches its own cmdline)

## architecture decisions (2026/8/18, from goal.md update)
- `layer` (proper noun): an INFINITE 2D plane, internally one `Space<Window>`; monitors are just
  VIEWPORTS into the layer (global layer, shown on all monitors)
- layer properties (user-editable): `window_layout_type` (tiling|stacked), `theme` ONLY
  (VFX_handler DELETED 2026/8/18: effects are user-level business, not a layer property)
- future option: a "Renderer layer" type (user program post-processes the accumulated frame
  between normal layers, like GIMP adjustment layers) -- fits Unix philosophy (blur/tint = one
  program each, chain by stacking); route = option 1 external process + shm fd (0.7.0 no ExportDma);
  frosted glass falls out naturally (blur wallpaper layer -> frosted backdrop)
- multi-layer model: `Vec<Layer>` rendered bottom-up; "switch two layers" = swap two Vec elements
- feat 1 (stacking) marked DONE [2026/8/17]: basic click-to-raise stacking; more features added gradually
- feat 5/6 renamed: infinity-screen -> infinity-layer (layer is infinite; translation moves viewport/layer)
- implementation order: feat 2 (dynamic tiling) FIRST on the existing single Space (no refactor yet),
  then layer abstraction (feats 3/4/5/6); pan input = touchpad gestures AND keyboard shortcuts
- tiling: grid-layout; each tiled window max size = smallest monitor size (layer infinite + viewable on
  any monitor -> tiles must fit smallest viewport)
- feat 2 (tiling) first version DONE (2026/8/18, commit 94a6561): horizontal row, insert beside focus +
  full relayout, drag-resize adjusts width share with neighbor compensation; move/raise ignored;
  old stacking grabs (move/resize) removed but kept in git history (commit 5718671) for stacked layers
- layout_type: the `window_layout_type` per-layer variable (tiling|stacked) is DECIDED to land WITH the
  Layer struct in feat 3, NOT as a global switch now (avoid rework); tiling.rs is written as a
  dispatchable layout strategy so a stacked branch just joins it in feat 3

## direction change (2026/8/18): pure stacking, no tiling
- goal.md updated by user: Dynamic-Tiling REMOVED from feature list; features renumbered
  (2=multi-layers overlay, 3=switch two layers, 4=infinity-layer, 5=infinity translation)
- tiling.rs + TilingResizeGrab + LayoutType deleted (commit 8af5a06); Layer = pure stacked
  container (labwc-style cascade placement, raise-to-top); git history keeps tiling (94a6561, 869fbce)
- layer properties now: theme + focused (bool; implemented as active_layer index, one focused layer)
- new proper noun `vscreen`: layer divided into rectangles of smallest-monitor size; window
  belongs to vscreen of its center point (border tie-break: right-top); infinity-layer groundwork

## maximize/minimize (2026/8/18, commit cbdcc89)
- vscreen implemented (src/vscreen.rs): layer grid of smallest-monitor cells;
  window center -> vscreen (goal.md border tie-break right-top: vertical border
  floor lands right, horizontal border y-1)
- maximize: move window into its vscreen rect + Maximized state + size;
  unmaximize: clear state+size (client restores own size, position stays)
- minimize: unmap_elem from space (auto skips render/hit-test/frame callbacks),
  saved position in Layer.minimized; unminimize API exists, restore trigger
  (command/taskbar/gesture) comes with command system
- fullscreen_request: NEXT commit (trait default still active)
