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

## smithay 0.7.0 api notes
- per-protocol macros `delegate_compositor!` `delegate_xdg_shell!` `delegate_shm!` (NOT `delegate_dispatch2!`, that is master-only)
- calloop 0.14: `Generic` lives at `calloop::generic::Generic`
- `delegate_xdg_shell!` forces `SeatHandler` impl (XdgPopup dispatch bound); no seat global needed
- winit backend requires GL: `winit::init_from_attributes::<GlesRenderer>(attrs)`
- `WinitEventLoop` is a calloop EventSource; render on `WinitEvent::Redraw`, `request_redraw()` after each frame
- `ShmState::new::<Self>(&dh, vec![])`; `CompositorState::new::<Self>(&dh)`; `XdgShellState::new::<Self>(&dh)`
- render: `render_elements_from_surface_tree` + `draw_render_elements`, `Transform::Flipped180` into winit window
- frame callbacks: `send_frames_surface_tree` via `SurfaceAttributes.frame_callbacks`

## pitfalls
- `cargo build --example` does NOT rebuild the main bin (stale `Hello, world!` binary bit us)
- `pkill -f "target/debug/terra-wm"` kills the calling shell too (pattern matches its own cmdline)
