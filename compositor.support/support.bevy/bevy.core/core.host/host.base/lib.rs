//! `BevyRuntime<S>`: one Bevy scene instance (app construction lives in
//! `compositor_support_bevy_core_boot_base`).

use std::sync::Arc;

use bevy::asset::Handle;
use bevy::image::Image;
use bevy::prelude::{App, World};
use compositor_support_bevy_core_scene_base::BevyScene;
use compositor_support_bevy_core_shared_base::SharedContext;

pub trait CommandHandler<C>: Send + 'static {
    fn handle(&mut self, command: &C);
}

impl<C, F> CommandHandler<C> for F
where
    F: FnMut(&C) + Send + 'static,
{
    fn handle(&mut self, command: &C) {
        (self)(command)
    }
}

pub struct BevyRuntime<S: BevyScene> {
    app: App,
    scene: S,
    output_handle: Handle<Image>,
    size_px: (u32, u32),
    scale_factor: f32,
    queued_commands: Vec<S::Command>,
    command_handler: Option<Box<dyn CommandHandler<S::Command>>>,
}

impl<S: BevyScene> std::fmt::Debug for BevyRuntime<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BevyRuntime")
            .field("size_px", &self.size_px)
            .field("scale_factor", &self.scale_factor)
            .field("queued_commands", &self.queued_commands.len())
            .finish()
    }
}

impl<S: BevyScene> BevyRuntime<S> {
    /// Construct a new runtime. `output_wgpu_texture` is a dmabuf-imported
    /// wgpu texture used as the scene's render target.
    pub fn new(
        scene: S,
        ctx: SharedContext,
        output_wgpu_texture: Arc<wgpu::Texture>,
        size_px: (u32, u32),
        scale_factor: f32,
    ) -> Self {
        let (app, output_handle) =
            compositor_support_bevy_core_boot_base::build_app(&scene, &ctx, output_wgpu_texture, size_px);

        Self {
            app,
            scene,
            output_handle,
            size_px,
            scale_factor,
            queued_commands: Vec::new(),
            command_handler: None,
        }
    }

    pub fn set_command_handler<H: CommandHandler<S::Command>>(&mut self, handler: H) {
        self.command_handler = Some(Box::new(handler));
    }
    pub fn clear_command_handler(&mut self) { self.command_handler = None; }
    pub fn queue_command(&mut self, command: S::Command) { self.queued_commands.push(command); }

    pub fn update(&mut self) {
        let commands = std::mem::take(&mut self.queued_commands);
        for command in commands {
            if let Some(handler) = self.command_handler.as_mut() {
                handler.handle(&command);
            }
            self.scene.apply_command(self.app.world_mut(), command);
        }
        self.app.update();
    }

    pub fn resize(&mut self, new_size_px: (u32, u32), scale_factor: f32) {
        self.size_px = new_size_px;
        self.scale_factor = scale_factor;
    }

    pub fn size_px(&self) -> (u32, u32) { self.size_px }
    pub fn scale_factor(&self) -> f32 { self.scale_factor }
    pub fn output_handle(&self) -> &Handle<Image> { &self.output_handle }
    pub fn world(&self) -> &World { self.app.world() }
    pub fn world_mut(&mut self) -> &mut World { self.app.world_mut() }
    pub fn scene(&self) -> &S { &self.scene }
    pub fn scene_mut(&mut self) -> &mut S { &mut self.scene }
}
