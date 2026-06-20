//! Ahead-of-time HDR composite shader compilation (M5).
//!
//! Compiles `composite_hdr.wgsl` to SPIR-V via naga at build time (no external
//! glslang/glslc; WGSL is naga's most robust frontend, and naga is the compiler
//! wgpu/Firefox ship). `hdr.rs` embeds the result with `include_bytes!`.
//! - composite_hdr.wgsl: vs_main + fs_tex + fs_solid (per-surface HDR composite)

use std::path::Path;

fn compile(name: &str) {
    let src_path = format!("shaders/{name}.wgsl");
    println!("cargo:rerun-if-changed={src_path}");
    let src = std::fs::read_to_string(&src_path).unwrap_or_else(|e| panic!("read {src_path}: {e}"));
    let module = naga::front::wgsl::parse_str(&src)
        .unwrap_or_else(|e| panic!("{name}.wgsl parse: {e:?}"));
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{name}.wgsl validate: {e:?}"));
    // Our WGSL geometry is authored in Vulkan's clip space directly (ported from
    // the GLSL shaders), so disable naga's WGSL→API Y-flip — otherwise the whole
    // HDR pass renders upside down.
    let mut opts = naga::back::spv::Options::default();
    opts.flags
        .remove(naga::back::spv::WriterFlags::ADJUST_COORDINATE_SPACE);
    let words = naga::back::spv::write_vec(&module, &info, &opts, None)
        .unwrap_or_else(|e| panic!("{name}.wgsl spv: {e:?}"));
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join(format!("{name}.spv"));
    std::fs::write(out, bytes).unwrap_or_else(|e| panic!("write {name}.spv: {e}"));
}

fn main() {
    compile("composite_hdr");
}
