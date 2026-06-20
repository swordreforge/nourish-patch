// overlay-runtime/src/driver.rs
use crate::driver_shell_notifier::OverlayNotifier;
use iced_core::renderer::Settings;
use iced_core::{
    Color, Event as IcedEvent, Font, Pixels, Size, Theme, mouse,
    renderer::Style,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use iced_wgpu::graphics::Shell;
use iced_wgpu::{Engine, Renderer, graphics::Viewport, wgpu};
use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use wayland_client::{Connection, Proxy, protocol::wl_surface::WlSurface};
use compositor_monitor_devtool_scene_base::app::CompositorSnapshot;
use compositor_monitor_devtool_scene_base::ui::{Message, Overlay};

pub struct IcedDriver {
    // Event/message queues we drain each frame.
    queued_events: Vec<IcedEvent>,
    // wgpu
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    surface_config: wgpu::SurfaceConfiguration,

    // iced
    renderer: Renderer,
    overlay: Overlay,
    cache: Cache,
    viewport: Viewport,

    queued_messages: Vec<Message>,

    // input bookkeeping
    cursor: mouse::Cursor,

    // Holds the connection alive for the surface lifetime.
    _conn: Connection,

    message_handler: Option<Box<dyn MessageHandler>>,
}

pub trait MessageHandler: Send + 'static {
    fn handle(&mut self, message: &Message);
}

impl IcedDriver {
    pub fn new(
        conn: &Connection,
        wl_surface: &WlSurface,
        size: (u32, u32),
        scale_factor: f64,
        snapshot: &CompositorSnapshot,
        redraw: Arc<AtomicBool>,
        invalidate: Arc<AtomicBool>,
    ) -> Self {
        // ─── wgpu init ──────────────────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            // CHECK: If there are presentation issues, pass display handle here.
            display: None,
        });

        let display_ptr = NonNull::new(conn.backend().display_ptr() as *mut _)
            .unwrap_or_else(|| abort!("wl_display pointer is null"));
        let surface_ptr =
            NonNull::new(wl_surface.id().as_ptr() as *mut _).unwrap_or_else(|| abort!("wl_surface pointer is null"));

        let raw_display = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display_ptr));
        let raw_window = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface_ptr));

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(raw_display),
                raw_window_handle: raw_window,
            })
        }
        .unwrap_or_else(|e| abort!("create wgpu surface: {e:?}"));

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            apply_limit_buckets: false,
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap_or_else(|e| abort!("no compatible wgpu adapter: {e:?}"));

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("overlay-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::default(),
            trace: wgpu::Trace::default(),
        }))
        .unwrap_or_else(|e| abort!("request wgpu device: {e:?}"));

        // ─── Surface configuration ──────────────────────────────────────
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps
                .alpha_modes
                .iter()
                .copied()
                .find(|m| matches!(m, wgpu::CompositeAlphaMode::PreMultiplied))
                .unwrap_or(caps.alpha_modes[0]),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // ─── iced init ──────────────────────────────────────────────────
        // Engine is consumed by Renderer in 0.14.
        let engine = Engine::new(
            &adapter,
            device.clone(),
            queue.clone(),
            surface_format,
            None,
            Shell::new(OverlayNotifier { invalidate, redraw }),
        );
        let renderer = Renderer::new(
            engine,
            Settings {
                default_font: Font::default(),
                default_text_size: Pixels::from(16),
            },
        );

        let viewport = Viewport::with_physical_size(Size::new(size.0, size.1), scale_factor as f32);

        // snapshot.clone()
        let overlay = Overlay::default();

        compositor_monitor_devtool_font_base::font::load();

        Self {
            message_handler: None,
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            surface,
            surface_format,
            surface_config,
            renderer,
            overlay,
            cache: Cache::default(),
            viewport,
            queued_events: Vec::new(),
            queued_messages: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
            _conn: conn.clone(),
        }
    }
    pub fn set_message_handler<H: MessageHandler>(&mut self, handler: H) {
        self.message_handler = Some(Box::new(handler));
    }

    // ─── Event ingest ───────────────────────────────────────────────────

    pub fn queue_event(&mut self, event: IcedEvent) {
        if let IcedEvent::Mouse(mouse::Event::CursorMoved { position }) = &event {
            self.cursor = mouse::Cursor::Available(*position);
        }
        if matches!(event, IcedEvent::Mouse(mouse::Event::CursorLeft)) {
            self.cursor = mouse::Cursor::Unavailable;
        }
        self.queued_events.push(event);
    }

    pub fn queue_message(&mut self, message: Message) {
        self.queued_messages.push(message);
    }

    // ─── Per-iteration update ───────────────────────────────────────────

    /// Drain queued events/messages, dispatch into a UserInterface, run
    /// `update` for any messages produced. Returns true if anything changed
    /// (in which case render_frame should be called).
    pub fn update(&mut self) -> bool {
        if self.queued_events.is_empty() && self.queued_messages.is_empty() {
            return false;
        }

        let logical = self.viewport.logical_size();
        let bounds = Size::new(logical.width, logical.height);

        let events = std::mem::take(&mut self.queued_events);
        let mut messages = std::mem::take(&mut self.queued_messages);

        // Phase 1: scoped block holds the UserInterface (and thus the borrow
        // of self.overlay). We do all UI work here, then drop the UI before
        // touching self.overlay mutably.
        let new_cache = {
            let cache = std::mem::take(&mut self.cache);
            let mut ui =
                UserInterface::build(self.overlay.view(), bounds, cache, &mut self.renderer);

            let (_state, _statuses) = ui.update(
                &events,
                self.cursor,
                &mut self.renderer,
                // &mut NullClipboard,
                &mut messages,
            );

            ui.into_cache()
        }; // ui dropped here; immutable borrow of self.overlay released

        self.cache = new_cache;

        // Phase 2: now we can mutate self.overlay freely.
        for message in messages.drain(..) {
            if let Some(handler) = self.message_handler.as_mut() {
                handler.handle(&message);
            }

            self.overlay.update(message);
        }

        true
    }

    // ─── Resize ─────────────────────────────────────────────────────────

    pub fn resize(&mut self, size: (u32, u32), scale_factor: f64) {
        self.surface_config.width = size.0.max(1);
        self.surface_config.height = size.1.max(1);
        self.surface.configure(&self.device, &self.surface_config);

        self.viewport =
            Viewport::with_physical_size(Size::new(size.0, size.1), scale_factor as f32);
    }

    // ─── Render & present ───────────────────────────────────────────────

    pub fn render_frame(&mut self) {
        // let frame = match self.surface.get_current_texture() {
        //     Ok(f) => f,
        //     Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
        //         self.surface.configure(&self.device, &self.surface_config);
        //         return;
        //     }
        //     Err(wgpu::SurfaceError::OutOfMemory) => return,
        //     Err(wgpu::SurfaceError::Timeout) => return,
        //     Err(wgpu::SurfaceError::Other) => return,
        // };

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) => f,
            wgpu::CurrentSurfaceTexture::Suboptimal(f) => {
                // Still usable, but reconfigure for next frame
                self.surface.configure(&self.device, &self.surface_config);
                f
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return,
        };



        // tracing::info!(
        //     "frame presented: size={:?}, format={:?}",
        //     self.surface_config.width,
        //     self.surface_config.format
        // );

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build the UI fresh from current state (post-update). The cache from
        // the prior update() preserves expensive layout/text-shaping work, so
        // this rebuild is cheap. We need a fresh UI here because the state
        // may have changed since update() ran (messages were applied).
        let logical = self.viewport.logical_size();
        let bounds = Size::new(logical.width, logical.height);
        let cache = std::mem::take(&mut self.cache);

        let mut ui = UserInterface::build(self.overlay.view(), bounds, cache, &mut self.renderer);

        ui.draw(
            &mut self.renderer,
            &Theme::Dark,
            &Style {
                text_color: Color::WHITE,
            },
            self.cursor,
        );

        self.cache = ui.into_cache();

        self.renderer.present(
            Some(Color::TRANSPARENT),
            self.surface_format,
            &view,
            &self.viewport,
        );

        self.queue.present(frame);
        // frame.present();
    }

    pub fn invalidate_layout(&mut self) {
        self.cache = iced_runtime::user_interface::Cache::default();
    }
}
