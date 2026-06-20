use smithay::backend::renderer::gles::{GlesFrame, GlesPixelProgram, GlesRenderer, GlesTexture, Uniform};
use smithay::backend::renderer::{Frame, Renderer, RendererSuper};
use smithay::utils::{Buffer as BufferCoord, Physical, Rectangle, Size};

/// Renderer-agnostic uniform values for the parallax background shader. The
/// GLES path ignores these (it uses the named `Uniform`s + its compiled
/// `GlesPixelProgram`); a renderer with a native background shader (Vulkan)
/// uses them to drive its own pipeline.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParallaxUniforms {
    pub resolution: [f32; 2],
    pub zoom: f32,
    pub time: f32,
    pub pan: [f32; 2],
    pub flow_offset: [f32; 2],
    pub lock_amount: f32,
    pub alpha: f32,
}

/// Per-renderer draw seam for scene elements that carry GLES-produced resources
/// (iced UI, bevy 3D, parallax pixel shader).
///
/// The trait is on the **renderer** `R` (a plain bound — `R: SceneDispatch`),
/// not on `R::Frame`, to avoid the GAT higher-ranked-lifetime limitation
/// (rust#100013) that a `for<'a,'b> R::Frame<'a,'b>: Trait` bound runs into at
/// the use site. The methods take the frame as a parameter.
///
/// - `GlesRenderer` implements it for real (renders the texture / runs the pixel

/// GLES body for `SceneDispatch::draw_prerendered_texture` (delegated from the
/// trait impl in dispatch.frame, which the orphan rule pins to the trait crate).
pub fn draw_prerendered_texture(
    frame: &mut GlesFrame<'_, '_>,
    texture: &GlesTexture,
    src: Rectangle<f64, BufferCoord>,
    dst: Rectangle<i32, Physical>,
    damage: &[Rectangle<i32, Physical>],
    alpha: f32,
) -> Result<(), <GlesRenderer as RendererSuper>::Error> {
    Frame::render_texture_from_to(
        frame, texture, src, dst, damage, &[], smithay::utils::Transform::Normal, alpha,
    )
}

/// GLES body for `SceneDispatch::draw_pixel_program`.
#[allow(clippy::too_many_arguments)]
pub fn draw_pixel_program(
    frame: &mut GlesFrame<'_, '_>,
    program: Option<&GlesPixelProgram>,
    src: Rectangle<f64, BufferCoord>,
    dst: Rectangle<i32, Physical>,
    size: Size<i32, BufferCoord>,
    damage: &[Rectangle<i32, Physical>],
    alpha: f32,
    uniforms: &[Uniform<'_>],
) -> Result<(), <GlesRenderer as RendererSuper>::Error> {
    // The program is `None` when the element was built while the compositor
    // preferred dmabuf/Vulkan (the GLES pixel program is skipped then). If the
    // GLES path is nonetheless reached — e.g. a runtime Vulkan→GLES fallback flips
    // the render path after the element was prepared — skip the parallax this frame
    // instead of crashing; the next prepare() rebuilds it with a compiled program.
    let Some(program) = program else {
        return Ok(());
    };
    frame.render_pixel_shader_to(program, src, dst, size, Some(damage), alpha, uniforms)
}
