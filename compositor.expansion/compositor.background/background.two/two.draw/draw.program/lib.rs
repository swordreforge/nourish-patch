//! Compiles the GLES pixel-shader program for the parallax background.
//
// Mountain: Clouds are just squares. Could be a more detailed square. Like in
// LowPoly games. Similarly the mountains can be better on that regard. Improve
// on the colors too. Feature: Time: the clouds should always move.
// Space Station: I actually meant a grounded space station. Perhaps with
// something ready to launch. Has brown and bluish purple touches.
// Feature: Time: whatever flows should always move.
// Space: We can keep current space. But with some changes:
// 1. Add a couple of more planets.
// 2. Remove the current thing that looks like a marker at the center- I think
//    it is meant to represent a station.
// 3. Feature: Time: A rocket that always seem to be moving upwards. After some
//    time, it'll reloop to the start. It has visible ignition engine.

use compositor_developer_debug_instance_record::abort;
use smithay::backend::renderer::gles::{GlesPixelProgram, GlesRenderer, UniformName, UniformType};

pub fn compile_program(renderer: &mut GlesRenderer) -> GlesPixelProgram {
    // UniformName is gone; we just pass the names directly as an array of strings.
    let program = renderer.compile_custom_pixel_shader(
        include_str!("../draw.element/shaders/spacev3.frag"),
        &[
            UniformName::new("u_time", UniformType::_1f),
            UniformName::new("u_lock_amount", UniformType::_1f),
            UniformName::new("u_pan", UniformType::_2f),
            UniformName::new("u_flow_offset", UniformType::_2f),
            UniformName::new("pan_velocity", UniformType::_2f),
            UniformName::new("u_zoom", UniformType::_1f),
            UniformName::new("u_resolution", UniformType::_2f),
        ],
    );

    let Ok(program) = program else {
        let err = program.err().unwrap();
        abort!("Failed to compile GLES shader {err:?}");
    };
    program
}
