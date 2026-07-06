// Y5_AA experiment composite for the textured (window + iced) arm. Separate
// image+sampler bindings (naga cannot emit combined image-samplers, so this
// cannot reuse the GLSL composite's combined `sampler2D`). ONE pipeline serves
// every AA mode: the sampler variant (bilinear / anisotropic / trilinear) is
// chosen per draw on the CPU, and `params.x` = supersample taps per axis lets
// the same shader do a straight sample (taps<=1) or an N×N supersample
// (taps>1). Mode is thus fully live-switchable with no pipeline rebuild.
//
// Only the textured `DrawOp` arm is routed here; solids and the parallax
// shader-pass (background) never touch it.

struct Push {
    dst: vec4<f32>,    // x, y, w, h in NDC
    src: vec4<f32>,    // u, v, w, h in UV
    color: vec4<f32>,  // (1,1,1,alpha) for textured
    // x = taps per axis (>=1); y = footprint spread; z = sharpen amount; w reserved
    params: vec4<f32>,
};
var<immediate> pc: Push;

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let corner = vec2<f32>(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let p = pc.dst.xy + corner * pc.dst.zw;
    out.uv = pc.src.xy + corner * pc.src.zw;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let taps = max(i32(pc.params.x), 1);
    let sharpen = pc.params.z;
    let lod_bias = pc.params.w;
    // No supersample and no sharpen: let the sampler do the work directly
    // (bilinear / aniso / trilinear-mip). `lod_bias` shifts the mip level
    // (negative → sharper/higher-res) for trilinear.
    if (taps <= 1 && sharpen <= 0.0) {
        return textureSampleBias(src_tex, src_samp, in.uv, lod_bias) * pc.color;
    }

    let px = dpdx(in.uv); // UV span of one output pixel, x
    let py = dpdy(in.uv); // UV span of one output pixel, y

    // Low-pass: N×N box supersample across the pixel footprint (× spread). This
    // removes minification aliasing/shimmer. `spread` 1.0 == a true one-pixel
    // box (crisp); >1 over-blurs.
    var col: vec4<f32>;
    if (taps > 1) {
        let spread = max(pc.params.y, 1.0);
        let dx = px * spread;
        let dy = py * spread;
        var acc = vec4<f32>(0.0);
        let inv = 1.0 / f32(taps);
        for (var j = 0; j < taps; j = j + 1) {
            for (var i = 0; i < taps; i = i + 1) {
                let fx = (f32(i) + 0.5) * inv - 0.5;
                let fy = (f32(j) + 0.5) * inv - 0.5;
                let off = fx * dx + fy * dy;
                acc = acc + textureSampleLevel(src_tex, src_samp, in.uv + off, 0.0);
            }
        }
        col = acc / f32(taps * taps);
    } else {
        col = textureSampleBias(src_tex, src_samp, in.uv, lod_bias);
    }

    // Sharpen: unsharp mask on the ALREADY-anti-aliased result. `wide` is a
    // cheap 4-tap neighbourhood average at ~2px; boosting (col - wide) raises
    // edge contrast (crisper text/borders) WITHOUT reintroducing source
    // aliasing, since it operates on the filtered image, not the raw texels.
    if (sharpen > 0.0) {
        let r = 2.0;
        let wide = 0.25 * (
            textureSampleLevel(src_tex, src_samp, in.uv + px * r, 0.0)
            + textureSampleLevel(src_tex, src_samp, in.uv - px * r, 0.0)
            + textureSampleLevel(src_tex, src_samp, in.uv + py * r, 0.0)
            + textureSampleLevel(src_tex, src_samp, in.uv - py * r, 0.0)
        );
        col = clamp(col + sharpen * (col - wide), vec4<f32>(0.0), vec4<f32>(1.0));
    }

    return col * pc.color;
}
