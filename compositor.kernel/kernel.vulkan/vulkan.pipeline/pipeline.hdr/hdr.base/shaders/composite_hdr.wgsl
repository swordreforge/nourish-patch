// HDR composite (M5, stage 1a). A WGSL port of the composite draw used ONLY when
// the HDR output path is active (the SDR path keeps the proven GLSL composite,
// untouched). Per-surface: an HDR-tagged surface (already PQ/BT.2020) passes
// through; everything else (SDR/Rec.709) is converted to the output color space
// (decode sRGB → linear, Rec.709→BT.2020, place at reference white, tone-map,
// PQ/HLG encode). All parameters travel in push constants (no UBO).

// Per-draw push constants — geometry + tuning merged into a single 112-byte block.
struct Push {
    dst: vec4<f32>,       // x, y, w, h in NDC
    src: vec4<f32>,       // u, v, w, h in UV
    color: vec4<f32>,     // rgba (solid) / (1,1,1,alpha) (textured)
    // x = source transfer (0 sRGB, 1 PQ, 2 HLG, 3 linear),
    // y = is_hdr passthrough (0/1), z/w reserved.
    surf: vec4<f32>,
    // -- tuning (formerly a uniform buffer) --
    enabled: f32,
    sdr_white_nits: f32,
    max_nits: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    gamut: f32,
    tone_map: f32,
    transfer: f32,
    gamma: f32,
    exposure: f32,
    _pad: f32,
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

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn pq_to_linear(e: vec3<f32>) -> vec3<f32> {
    let m1 = 0.1593017578125;
    let m2 = 78.84375;
    let c1 = 0.8359375;
    let c2 = 18.8515625;
    let c3 = 18.6875;
    let ep = pow(max(e, vec3<f32>(0.0)), vec3<f32>(1.0 / m2));
    let num = max(ep - vec3<f32>(c1), vec3<f32>(0.0));
    let den = vec3<f32>(c2) - c3 * ep;
    return pow(num / den, vec3<f32>(1.0 / m1)) * 10000.0;
}

fn rec709_to_bt2020(c: vec3<f32>) -> vec3<f32> {
    let r = dot(vec3<f32>(0.627404, 0.329283, 0.043313), c);
    let g = dot(vec3<f32>(0.069097, 0.919540, 0.011362), c);
    let b = dot(vec3<f32>(0.016391, 0.088013, 0.895595), c);
    return vec3<f32>(r, g, b);
}

fn luma2020(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2627, 0.6780, 0.0593));
}

fn pq_encode(nits: vec3<f32>) -> vec3<f32> {
    let m1 = 0.1593017578125;
    let m2 = 78.84375;
    let c1 = 0.8359375;
    let c2 = 18.8515625;
    let c3 = 18.6875;
    let y = clamp(nits / 10000.0, vec3<f32>(0.0), vec3<f32>(1.0));
    let ym1 = pow(y, vec3<f32>(m1));
    return pow((vec3<f32>(c1) + c2 * ym1) / (vec3<f32>(1.0) + c3 * ym1), vec3<f32>(m2));
}

fn tone_reinhard(nits: vec3<f32>, peak: f32) -> vec3<f32> {
    let p = max(peak, 1.0);
    return nits * (vec3<f32>(1.0) + nits / vec3<f32>(p * p)) / (vec3<f32>(1.0) + nits / vec3<f32>(p));
}

fn to_output(rgb_in: vec3<f32>, alpha: f32) -> vec4<f32> {
    if (pc.surf.y > 0.5) {
        return vec4<f32>(rgb_in, alpha);
    }
    var lin: vec3<f32>;
    let tf = pc.surf.x;
    if (tf >= 0.5 && tf < 1.5) {
        lin = pq_to_linear(rgb_in) / max(pc.sdr_white_nits, 1.0);
    } else if (tf >= 2.5) {
        lin = rgb_in;
    } else {
        lin = srgb_to_linear(rgb_in);
    }
    lin = lin * max(pc.exposure, 0.0) * max(pc.brightness, 0.0);
    lin = pow(max(lin, vec3<f32>(0.0)), vec3<f32>(max(pc.gamma, 0.01)));
    lin = (lin - vec3<f32>(0.5)) * pc.contrast + vec3<f32>(0.5);
    lin = max(lin, vec3<f32>(0.0));
    lin = mix(lin, rec709_to_bt2020(lin), clamp(pc.gamut, 0.0, 1.0));
    let l = luma2020(lin);
    lin = max(mix(vec3<f32>(l), lin, pc.saturation), vec3<f32>(0.0));
    var nits = lin * max(pc.sdr_white_nits, 1.0);
    if (pc.tone_map >= 0.5) {
        nits = tone_reinhard(nits, pc.max_nits);
    } else {
        nits = min(nits, vec3<f32>(max(pc.max_nits, 1.0)));
    }
    return vec4<f32>(pq_encode(nits), alpha);
}

@fragment
fn fs_tex(in: VsOut) -> @location(0) vec4<f32> {
    let s = textureSample(src_tex, src_samp, in.uv) * pc.color;
    if (pc.enabled < 0.5) {
        return s;
    }
    return to_output(s.rgb, s.a);
}

@fragment
fn fs_solid(in: VsOut) -> @location(0) vec4<f32> {
    if (pc.enabled < 0.5) {
        return pc.color;
    }
    return to_output(pc.color.rgb, pc.color.a);
}
