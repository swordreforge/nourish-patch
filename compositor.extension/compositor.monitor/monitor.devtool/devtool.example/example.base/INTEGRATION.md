# Iced-DMABUF integration walkthrough

This is the operational counterpart to your existing `integration.md`. Same flow, applied to the multi-instance Iced subsystem.

## Once at startup

```rust
// On a worker thread, mirroring the Bevy WGPU init pattern:
let (tx, rx) = mpsc::channel();
std::thread::spawn(move || {
    let result = compositor_monitor_runtime_surface_base::create_wgpu_vulkan_context();
    let _ = tx.send(result);
});
state.iced_wgpu_context_init = Some(rx);
```

## When the GLES renderer is available

```rust
// Receive the context (poll once per iteration until ready):
if state.iced_wgpu_context.is_none() {
    if let Some(rx) = &state.iced_wgpu_context_init {
        match rx.try_recv() {
            Ok(Ok(ctx)) => {
                state.iced_wgpu_context = Some(Arc::new(ctx));
                state.iced_wgpu_context_init = None;
            }
            Ok(Err(e)) => {
                tracing::error!("iced wgpu init failed: {e:?}");
                state.iced_wgpu_context_init = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                state.iced_wgpu_context_init = None;
            }
        }
    }
}

// Once we have it AND we have a gles renderer:
if state.iced_registry.is_none() && state.iced_wgpu_context.is_some() {
    let wgpu_ctx = state.iced_wgpu_context.as_ref().unwrap().clone();
    let shared_engine = SharedEngine::new(
        &wgpu_ctx.adapter,
        Arc::new(wgpu_ctx.device.clone()),
        Arc::new(wgpu_ctx.queue.clone()),
        compositor_monitor_runtime_surface_base::TEXTURE_FORMAT,
        EngineSettings::default(),
    );
    state.iced_registry = Some(IcedRegistry::new(shared_engine, wgpu_ctx));
}
```

## Spawning an Iced instance

```rust
let handle = state.iced_registry.as_mut().unwrap().create(
    CounterUi::default(),
    gles_renderer,
    Point::from((40, 40)),
    Size::from((360, 220)),
)?;

// Install the message observer.
state.iced_registry.as_mut().unwrap()
    .instance_mut(handle).unwrap()
    .runtime_mut()
    .set_message_handler(|msg: &OutgoingMessage| {
        tracing::info!("iced→compositor: {:?}", msg);
    });
```

## In the render callback (where you currently produce BevyBackgroundElement)

```rust
let iced_elements = state.iced_registry
    .as_mut()
    .unwrap()
    .render_all(gles_renderer)
    .unwrap_or_default();

// Add them to your render list alongside windows / Bevy bg / shader bg.
for elem in &iced_elements {
    elements.push(MyRenderElement::Iced(elem.clone()));
}
```

## Pointer input (BEFORE forwarding to wayland clients)

```rust
fn handle_pointer_motion(state: &mut State, point: Point<f64, Physical>) {
    let registry = state.iced_registry.as_mut().unwrap();
    if registry.dispatch_pointer_at(point).is_some() {
        // Swallow — pointer is over an iced UI.
        return;
    }
    // Otherwise, normal wayland routing.
    state.seat.pointer().motion(...)
}
```

## Pointer buttons / scroll

```rust
fn handle_pointer_button(state: &mut State, linux_code: u32, pressed: bool) {
    let registry = state.iced_registry.as_mut().unwrap();
    if let Some(target) = registry.pointer_target() {
        let event = if pressed {
            compositor_monitor_compositor_iced_base::input::button_pressed(linux_code)
        } else {
            compositor_monitor_compositor_iced_base::input::button_released(linux_code)
        };
        if let Some(e) = event {
            let _ = registry.dispatch_event(target, e);
        }
        return;  // swallow
    }
    // Otherwise, normal wayland routing.
}
```

## Keyboard

```rust
fn handle_key(state: &mut State, keysym: u32, utf8: Option<&str>, pressed: bool, repeat: bool) {
    // Whatever logic determines which iced instance has keyboard focus.
    if let Some(focused) = state.focused_iced {
        let mods = state.compute_iced_mods();
        if let Some(e) = compositor_monitor_compositor_iced_base::input::keyboard_event(
            keysym, utf8, pressed, repeat, mods,
        ) {
            let _ = state.iced_registry.as_mut().unwrap()
                .dispatch_event(focused, e);
            return;  // swallow
        }
    }
    // Normal wayland routing.
}
```

## Compositor → UI: dispatching messages in

```rust
// Anywhere in your compositor, by handle:
state.iced_registry.as_mut().unwrap()
    .dispatch_message(handle, OutgoingMessage::SmithayTick(state.frame_count));

// Or by id (after a hit-test, etc.):
state.iced_registry.as_mut().unwrap()
    .dispatch_event(handle_id, IcedEvent::Window(...));
```

## Resizing

```rust
// Request: visible to subsequent reads only after apply.
state.iced_registry.as_mut().unwrap()
    .request_resize(handle, Size::from((400, 300)));

// Applied automatically by render_all(); or explicitly if you split phases:
state.iced_registry.as_mut().unwrap()
    .apply_pending_resizes(gles_renderer)?;
```

## Destroying

```rust
state.iced_registry.as_mut().unwrap().destroy(handle);
// Or by id:
state.iced_registry.as_mut().unwrap().destroy_by_id(handle_id);
```
