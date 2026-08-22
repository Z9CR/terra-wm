//! DRM/udev backend: run terra-wm natively on a TTY.
//!
//! Simplified from anvil's udev backend: single GPU, first connected
//! connector, no dmabuf feedback / presentation / debug. The winit backend
//! stays available for nested development.

use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc, time::Duration};

use smithay::{
    backend::{
        allocator::{
            Fourcc,
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
        },
        drm::{
            DrmDevice, DrmDeviceFd, DrmEvent, DrmNode, NodeType,
            compositor::FrameFlags,
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
        },
        egl::{EGLContext, EGLDisplay, context::ContextPriority},
        input::InputEvent,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{element::surface::WaylandSurfaceRenderElement, gles::GlesRenderer},
        session::{Event as SessionEvent, Session, libseat::LibSeatSession},
        udev::{UdevBackend, UdevEvent, all_gpus, primary_gpu},
    },
    desktop::space::space_render_elements,
    reexports::{
        calloop::{
            EventLoop, LoopHandle,
            timer::{TimeoutAction, Timer},
        },
        drm::control::{ModeTypeFlags, connector, crtc},
        input::{DeviceCapability, Libinput},
        rustix::fs::OFlags,
        wayland_server::{Display, backend::GlobalId},
    },
    utils::DeviceFd,
};
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};

use crate::{render::post_render, state::TerraWm};

type DrmAlloc = GbmAllocator<DrmDeviceFd>;
type DrmExport = GbmFramebufferExporter<DrmDeviceFd>;
type DrmMgr = DrmOutputManager<DrmAlloc, DrmExport, (), DrmDeviceFd>;
type DrmSurf = DrmOutput<DrmAlloc, DrmExport, (), DrmDeviceFd>;

struct SurfaceData {
    output: smithay::output::Output,
    _global: GlobalId,
    drm_output: DrmSurf,
}

pub struct DrmData {
    session: LibSeatSession,
    _primary_gpu: DrmNode,
    output_manager: Option<DrmMgr>,
    renderer: Option<GlesRenderer>,
    scanner: DrmScanner,
    surfaces: HashMap<crtc::Handle, SurfaceData>,
}

impl DrmData {
    fn new(session: LibSeatSession, primary_gpu: DrmNode) -> Self {
        Self {
            session,
            _primary_gpu: primary_gpu,
            output_manager: None,
            renderer: None,
            scanner: DrmScanner::new(),
            surfaces: HashMap::new(),
        }
    }

    fn device_added(
        &mut self,
        drm_data: &Rc<RefCell<DrmData>>,
        node: DrmNode,
        path: &Path,
        state: &mut TerraWm,
        handle: &LoopHandle<TerraWm>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.output_manager.is_some() {
            return Ok(());
        }

        let fd = self.session.open(
            path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )?;
        let fd = DrmDeviceFd::new(DeviceFd::from(fd));

        let (device, notifier) = DrmDevice::new(fd.clone(), true)?;
        let gbm = GbmDevice::new(fd)?;

        let drm2 = Rc::clone(drm_data);
        let handle2 = handle.clone();
        handle.insert_source(notifier, move |event, metadata, state| {
            let mut drm = drm2.borrow_mut();
            match event {
                DrmEvent::VBlank(crtc) => drm.frame_finish(&drm2, crtc, state, &handle2),
                DrmEvent::Error(error) => tracing::error!(?error, "drm error"),
            }
            let _ = metadata;
        })?;

        let display = unsafe { EGLDisplay::new(gbm.clone())? };
        let context = EGLContext::new_with_priority(&display, ContextPriority::High)?;
        let renderer = unsafe { GlesRenderer::new(context)? };
        tracing::info!("initialized GL renderer for drm");

        let render_formats = renderer.egl_context().dmabuf_render_formats().clone();
        let color_formats = [Fourcc::Abgr8888, Fourcc::Argb8888];

        let allocator = GbmAllocator::new(
            gbm.clone(),
            GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
        );
        let exporter = GbmFramebufferExporter::new(gbm.clone(), Some(node).into());
        let output_manager = DrmOutputManager::new(
            device,
            allocator,
            exporter,
            Some(gbm),
            color_formats,
            render_formats,
        );

        self.output_manager = Some(output_manager);
        self.renderer = Some(renderer);

        self.device_changed(drm_data, node, state, handle);
        Ok(())
    }

    fn device_changed(
        &mut self,
        drm_data: &Rc<RefCell<DrmData>>,
        node: DrmNode,
        state: &mut TerraWm,
        handle: &LoopHandle<TerraWm>,
    ) {
        let Some(output_manager) = &mut self.output_manager else {
            return;
        };
        let scan_result = match self.scanner.scan_connectors(output_manager.device()) {
            Ok(result) => result,
            Err(err) => {
                tracing::warn!(?err, "failed to scan connectors");
                return;
            }
        };

        for event in scan_result.iter() {
            match event {
                DrmScanEvent::Connected { connector, crtc } => {
                    if let Some(crtc) = crtc {
                        self.connector_connected(drm_data, node, connector, crtc, state, handle);
                    }
                }
                DrmScanEvent::Disconnected { connector, crtc } => {
                    if let Some(crtc) = crtc {
                        self.connector_disconnected(node, connector, crtc, state);
                    }
                }
                _ => (),
            }
        }
    }

    fn connector_connected(
        &mut self,
        drm_data: &Rc<RefCell<DrmData>>,
        _node: DrmNode,
        connector: connector::Info,
        crtc: crtc::Handle,
        state: &mut TerraWm,
        handle: &LoopHandle<TerraWm>,
    ) {
        if self.surfaces.contains_key(&crtc) {
            return;
        }
        let Some(output_manager) = &mut self.output_manager else {
            return;
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let output_name = format!(
            "{}-{}",
            connector.interface().as_str(),
            connector.interface_id()
        );
        tracing::info!(?crtc, output = output_name.as_str(), "connector connected");

        let mode_id = connector
            .modes()
            .iter()
            .position(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
            .unwrap_or(0);
        let drm_mode = connector.modes()[mode_id];
        let wl_mode = smithay::output::Mode::from(drm_mode);

        let (phys_w, phys_h) = connector.size().unwrap_or((0, 0));
        let output = smithay::output::Output::new(
            output_name,
            smithay::output::PhysicalProperties {
                size: (phys_w as i32, phys_h as i32).into(),
                subpixel: connector.subpixel().into(),
                make: "Unknown".into(),
                model: "Unknown".into(),
                serial_number: "Unknown".into(),
            },
        );
        let global = output.create_global::<TerraWm>(&state.display_handle);

        output.set_preferred(wl_mode);
        output.change_current_state(Some(wl_mode), None, None, Some((0, 0).into()));

        // replace the winit placeholder output so all shared logic uses this one
        state.output = output.clone();
        for layer in &mut state.layer_stack {
            layer.space.map_output(&output, state.view_offset);
        }

        let mut planes = match output_manager.device().planes(&crtc) {
            Ok(planes) => planes,
            Err(err) => {
                tracing::warn!(?err, "failed to get crtc planes");
                return;
            }
        };
        // skip overlay planes for now (single primary plane is enough)
        planes.overlay = vec![];

        let drm_output = match output_manager
            .lock()
            .initialize_output::<_, WaylandSurfaceRenderElement<GlesRenderer>>(
                crtc,
                drm_mode,
                &[connector.handle()],
                &output,
                Some(planes),
                renderer,
                &DrmOutputRenderElements::default(),
            ) {
            Ok(drm_output) => drm_output,
            Err(err) => {
                tracing::warn!(?err, "failed to initialize drm output");
                return;
            }
        };

        self.surfaces.insert(
            crtc,
            SurfaceData {
                output: output.clone(),
                _global: global,
                drm_output,
            },
        );

        // kick off rendering
        let drm_rc = Rc::clone(drm_data);
        let handle2 = handle.clone();
        handle.insert_idle(move |state| {
            drm_rc.borrow_mut().render(&drm_rc, crtc, state, &handle2);
        });
    }

    fn connector_disconnected(
        &mut self,
        _node: DrmNode,
        _connector: connector::Info,
        crtc: crtc::Handle,
        _state: &mut TerraWm,
    ) {
        tracing::info!(?crtc, "connector disconnected");
        if let Some(surface) = self.surfaces.remove(&crtc) {
            let _ = surface;
        }
    }

    fn device_removed(&mut self, _node: DrmNode) {
        tracing::info!("drm device removed");
    }

    fn render(
        &mut self,
        drm_data: &Rc<RefCell<DrmData>>,
        crtc: crtc::Handle,
        state: &mut TerraWm,
        handle: &LoopHandle<TerraWm>,
    ) {
        let Some(surface) = self.surfaces.get_mut(&crtc) else {
            return;
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let output = surface.output.clone();
        let scale = output.current_scale().fractional_scale();

        let spaces = state.layer_stack.iter().rev().map(|layer| &layer.space);
        let elements = match space_render_elements(renderer, spaces, &output, 1.0) {
            Ok(elements) => elements,
            Err(err) => {
                tracing::warn!(?err, "no mode for output");
                return;
            }
        };

        let (rendered, _states) = match surface.drm_output.render_frame(
            renderer,
            &elements,
            [0.1, 0.1, 0.1, 1.0],
            FrameFlags::empty(),
        ) {
            Ok(result) => (!result.is_empty, result.states),
            Err(err) => {
                tracing::warn!(?err, "error during rendering");
                return;
            }
        };

        if rendered {
            if let Err(err) = surface.drm_output.queue_frame(()) {
                tracing::warn!(?err, "failed to queue frame");
                return;
            }
            post_render(state, &output);
        }

        let _ = scale;
        if !rendered {
            self.schedule_render(drm_data, crtc, state, handle);
        }
    }

    fn frame_finish(
        &mut self,
        drm_data: &Rc<RefCell<DrmData>>,
        crtc: crtc::Handle,
        state: &mut TerraWm,
        handle: &LoopHandle<TerraWm>,
    ) {
        let Some(surface) = self.surfaces.get_mut(&crtc) else {
            return;
        };
        if let Err(err) = surface.drm_output.frame_submitted() {
            tracing::warn!(?err, "frame_submitted failed");
        }
        self.schedule_render(drm_data, crtc, state, handle);
    }

    fn schedule_render(
        &self,
        drm_data: &Rc<RefCell<DrmData>>,
        crtc: crtc::Handle,
        _state: &mut TerraWm,
        handle: &LoopHandle<TerraWm>,
    ) {
        let Some(surface) = self.surfaces.get(&crtc) else {
            return;
        };
        let Some(frame_duration) = surface
            .output
            .current_mode()
            .map(|mode| Duration::from_secs_f64(1_000f64 / mode.refresh as f64))
        else {
            return;
        };
        let delay = Duration::from_secs_f64(frame_duration.as_secs_f64() * 0.6);
        let _ = delay;

        let drm2 = Rc::clone(drm_data);
        let handle2 = handle.clone();
        let timer = Timer::from_duration(frame_duration);
        let _ = handle
            .insert_source(timer, move |_, _, state| {
                drm2.borrow_mut().render(&drm2, crtc, state, &handle2);
                TimeoutAction::Drop
            })
            .map_err(|err| tracing::error!(?err, "failed to schedule render"));
    }
}

pub fn run_udev(command: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop: EventLoop<TerraWm> = EventLoop::try_new()?;
    let display: Display<TerraWm> = Display::new()?;
    let mut state = TerraWm::new(&mut event_loop, display);
    let handle = event_loop.handle();

    let (session, notifier) = LibSeatSession::new()?;
    let seat_name = session.seat().to_string();
    tracing::info!(seat = seat_name.as_str(), "session acquired");

    let primary_gpu = primary_gpu(&seat_name)?
        .and_then(|path| {
            DrmNode::from_path(path)
                .ok()?
                .node_with_type(NodeType::Render)?
                .ok()
        })
        .or_else(|| {
            all_gpus(&seat_name)
                .ok()?
                .into_iter()
                .find_map(|path| DrmNode::from_path(path).ok())
        })
        .ok_or("no DRM device found")?;
    tracing::info!(?primary_gpu, "using primary gpu");

    let udev_backend = UdevBackend::new(&seat_name)?;

    let drm_data = Rc::new(RefCell::new(DrmData::new(session, primary_gpu)));

    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        drm_data.borrow().session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(&seat_name)
        .map_err(|_| "failed to assign udev seat")?;
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    handle.insert_source(libinput_backend, move |event, _, state| {
        match &event {
            InputEvent::DeviceAdded { device } => {
                if device.has_capability(DeviceCapability::Keyboard) {
                    tracing::info!("keyboard device added");
                }
            }
            InputEvent::DeviceRemoved { device } => {
                if device.has_capability(DeviceCapability::Keyboard) {
                    tracing::info!("keyboard device removed");
                }
            }
            _ => (),
        }
        state.process_input_event(event)
    })?;

    let drm2 = Rc::clone(&drm_data);
    let mut libinput2 = libinput_context.clone();
    handle.insert_source(notifier, move |event, _, _state| {
        let mut drm = drm2.borrow_mut();
        match event {
            SessionEvent::PauseSession => {
                libinput2.suspend();
                if let Some(mgr) = &mut drm.output_manager {
                    mgr.pause();
                }
            }
            SessionEvent::ActivateSession => {
                libinput2.resume().ok();
                if let Some(mgr) = &mut drm.output_manager {
                    mgr.lock().activate(false).ok();
                }
                for crtc in drm.surfaces.keys().cloned().collect::<Vec<_>>() {
                    let _ = crtc;
                }
            }
        }
    })?;

    // scan the initial device list before moving udev_backend into the loop
    let primary_node = primary_gpu
        .node_with_type(NodeType::Primary)
        .and_then(|node| node.ok());
    let primary_device = udev_backend.device_list().find(|(device_id, _)| {
        primary_node
            .map(|primary_node| *device_id == primary_node.dev_id())
            .unwrap_or(false)
            || *device_id == primary_gpu.dev_id()
    });

    if let Some((device_id, path)) = primary_device {
        let node = DrmNode::from_dev_id(device_id)?;
        drm_data
            .borrow_mut()
            .device_added(&drm_data, node, path, &mut state, &handle)?;
    }

    let drm3 = Rc::clone(&drm_data);
    let handle2 = handle.clone();
    handle.insert_source(udev_backend, move |event, _, state| {
        let mut drm = drm3.borrow_mut();
        match event {
            UdevEvent::Added { device_id, path } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    if let Err(err) = drm.device_added(&drm3, node, &path, state, &handle2) {
                        tracing::error!(?err, "failed to init drm device");
                    }
                }
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    drm.device_changed(&drm3, node, state, &handle2);
                }
            }
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    drm.device_removed(node);
                }
            }
        }
    })?;

    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket_name) };
    if let Some(command) = command {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        if let Err(e) = cmd.spawn() {
            tracing::warn!(error = %e, "failed to spawn client");
        }
    }

    event_loop.run(None, &mut state, |_| {})?;
    Ok(())
}
