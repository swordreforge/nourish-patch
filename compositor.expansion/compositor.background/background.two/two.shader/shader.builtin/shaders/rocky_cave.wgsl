// Built-in background: "Rocky Cave" — the *inside* companion to Rocky Galaxy.
// You're deep in a cave of the rocky world, looking out toward a bright exit. The
// cave is a wall of faceted low-poly rock chunks; the opening follows their angular
// edges, and daylight glows through it. Same quiet, unobtrusive mood.
//
// Design notes (kept deliberately restful):
//   * The dark frame is a Voronoi array of flat-shaded rock facets (a low-poly
//     look), each catching the light differently, with thin crevices between them.
//   * The exit is carved out of that array — a chunk is "open" when its facet
//     falls inside the mouth, so the hole is angular, not a smooth oval. Daylight
//     and a distant ridge show through, and a warm rim lights the near rocks.
//   * Dust drifts in the light. The rock frame parallaxes more than the far view.
//
// Author-exposed knobs (parsed from the `@prop` lines below → params slots):
// @prop drift_speed float default=1.0 min=0.0 max=6.0 label="Drift & dust" group="Cave"
// @prop dust_density float default=1.0 min=0.0 max=2.0 label="Dust density" group="Cave"
// @prop daylight float default=1.0 min=0.0 max=2.0 label="Daylight" group="Cave"
// @prop vignette float default=0.0 min=0.0 max=1.0 label="Vignette amount" group="Cave"
// @prop vignette_radius float default=1.12 min=0.5 max=2.0 label="Vignette radius" group="Cave"
// @prop vignette_softness float default=0.6 min=0.05 max=2.0 label="Vignette softness" group="Cave"

struct Push {
    res_zoom_time: vec4<f32>,
    pan_flow: vec4<f32>,
    lock_alpha: vec4<f32>,
    params: array<vec4<f32>, 4>,
};
var<immediate> pc: Push;

struct VsOut { @builtin(position) pos: vec4<f32> };

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    let uv = vec2<f32>(f32((vid << 1u) & 2u), f32(vid & 2u));
    var o: VsOut;
    o.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    return o;
}

fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y, p3.z, p3.x) + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z);
}
fn hash2(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    p3 = p3 + dot(p3, vec3<f32>(p3.y, p3.z, p3.x) + vec3<f32>(33.33));
    return fract(vec2<f32>((p3.x + p3.y) * p3.z, (p3.x + p3.z) * p3.y));
}
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y,
    );
}
fn fbm(p_in: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var p = p_in;
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * noise(p);
        p = p * 2.0;
        a = a * 0.5;
    }
    return v;
}

// Voronoi facet lookup: nearest jittered cell (f1), the runner-up (f2, for edges),
// the winning cell's integer coord and its centre point (in `p` space).
struct Voro { f1: f32, f2: f32, cell: vec2<f32>, center: vec2<f32> };
fn voronoi(p: vec2<f32>) -> Voro {
    let ip = floor(p);
    let fp = p - ip;
    var r: Voro;
    r.f1 = 8.0;
    r.f2 = 8.0;
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let g = vec2<f32>(f32(x), f32(y));
            let o = g + hash2(ip + g);
            let d = length(o - fp);
            if (d < r.f1) {
                r.f2 = r.f1; r.f1 = d;
                r.cell = ip + g;
                r.center = ip + o;
            } else if (d < r.f2) {
                r.f2 = d;
            }
        }
    }
    return r;
}

// The bright world seen through the mouth: a warm daylit sky over hazy ridges.
fn exterior(p: vec2<f32>, daylight: f32) -> vec3<f32> {
    let t = clamp(p.y * 1.4 + 0.5, 0.0, 1.0);
    var e = mix(vec3<f32>(0.34, 0.24, 0.15), vec3<f32>(0.24, 0.30, 0.38), t);
    e = e + vec3<f32>(0.28, 0.19, 0.09) * exp(-((p.x - 0.12) * (p.x - 0.12) + (p.y + 0.06) * (p.y + 0.06)) * 4.0);
    let r1 = 0.02 + 0.06 * fbm(vec2<f32>(p.x * 1.4 + 4.0, 0.0));
    e = mix(e, vec3<f32>(0.24, 0.19, 0.15), smoothstep(0.01, -0.01, p.y - r1));
    let r2 = -0.10 + 0.05 * fbm(vec2<f32>(p.x * 2.3 + 9.0, 0.0));
    e = mix(e, vec3<f32>(0.16, 0.12, 0.10), smoothstep(0.01, -0.01, p.y - r2));
    return e * (0.75 + 0.45 * daylight);
}

// The mouth radius at angle `a` around its centre — irregular but smooth.
fn mouth_radius(a: f32) -> f32 {
    return 0.34 + 0.10 * fbm(vec2<f32>(cos(a), sin(a)) * 2.2 + 5.0) + 0.045 * sin(a * 3.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let frag = in.pos.xy;
    let res = pc.res_zoom_time.xy;
    let zoom = pc.res_zoom_time.z;
    let time = pc.res_zoom_time.w;
    let pan_in = pc.pan_flow.xy;
    let lock_amount = pc.lock_alpha.x;
    let alpha = pc.lock_alpha.y;

    let drift = pc.params[0].x;
    let dust_density = pc.params[0].y;
    let daylight = pc.params[0].z;
    let vignette = pc.params[0].w;
    let vig_radius = pc.params[1].x;
    let vig_softness = pc.params[1].y;

    var uv = (frag - 0.5 * res) / res.y;
    let screen_uv = uv;
    uv = uv / zoom;
    let pan = vec2<f32>(pan_in.x, -pan_in.y);

    let oc = vec2<f32>(-0.05, -0.06);
    // Rock-facet field in the (near-parallaxed) frame space.
    let scale = 5.5;
    let fp = (uv - pan * 0.0006) * scale;
    let v = voronoi(fp);

    // Is this facet part of the open mouth? Test the facet's CENTRE against the
    // mouth outline, so the hole follows the angular rock edges.
    let crel = v.center / scale - oc;
    let crad = length(crel * vec2<f32>(1.0, 1.28));
    let cang = atan2(crel.y, crel.x);
    let is_open = crad < mouth_radius(cang);

    var col: vec3<f32>;
    if (is_open) {
        // Daylight through the mouth (far parallax).
        col = exterior(uv * 0.85 - pan * 0.00018, daylight);
    } else {
        // Flat-shaded low-poly rock facet: a constant pseudo-normal per cell lit by
        // a fixed key, so each chunk catches a different flat tone.
        let nrm = normalize(vec3<f32>((hash2(v.cell) - 0.5) * 1.6, 0.85));
        let key = normalize(vec3<f32>(-0.45, 0.55, 0.55));
        let sh = clamp(dot(nrm, key), 0.0, 1.0);
        var rock = mix(vec3<f32>(0.014, 0.011, 0.010), vec3<f32>(0.075, 0.06, 0.05), sh * sh);
        // Thin dark crevices between facets (Voronoi edges).
        let edge = smoothstep(0.0, 0.05, v.f2 - v.f1);
        rock = rock * (0.35 + 0.65 * edge);
        // Warm rim: only the facets nearest the mouth catch a little spill light.
        let rim = smoothstep(0.2, 0.0, crad - mouth_radius(cang));
        rock = rock + vec3<f32>(0.32, 0.21, 0.10) * rim * rim * (0.45 + 0.4 * daylight);
        col = rock;
    }

    // A soft shaft of light reaching in from the mouth (independent of facets).
    let mrel = (uv - pan * 0.0006) - oc;
    let mrad = length(mrel * vec2<f32>(1.0, 1.28));
    let mr = mouth_radius(atan2(mrel.y, mrel.x));
    col = col + vec3<f32>(0.12, 0.08, 0.04) * exp(-(mrad - mr) * 2.2) * f32(!is_open) * smoothstep(1.1, mr, mrad) * (0.4 + 0.4 * daylight);

    // Dust motes drifting in the light near the mouth.
    for (var i = 0; i < 2; i = i + 1) {
        let depth = 1.0 + f32(i) * 0.7;
        let sp = vec2<f32>(uv.x * (10.0 / depth) + pan.x * 0.0011 * depth + time * 0.03 * drift / depth,
                           uv.y * (10.0 / depth) - time * 0.02 * drift / depth + pan.y * 0.0011 * depth);
        let id = floor(sp);
        let f = fract(sp) - 0.5;
        let h = hash(id + f32(i) * 29.0);
        if (h > 1.0 - 0.05 * dust_density) {
            let dd = length(f);
            let near_light = smoothstep(1.0, mr - 0.1, mrad);
            col = col + vec3<f32>(0.5, 0.36, 0.2) * smoothstep(0.07, 0.0, dd) * (0.5 + 0.5 * sin(time + h * 30.0)) * near_light / (depth * 3.0);
        }
    }

    // Lock-screen ease: let the daylight dim toward dusk.
    var l = clamp(lock_amount, 0.0, 1.0);
    l = l * l * (3.0 - 2.0 * l);
    col = mix(col, col * 0.5 + vec3<f32>(0.008, 0.006, 0.005), l);

    let vig = smoothstep(vig_radius, vig_radius - vig_softness, length(screen_uv));
    col = col * mix(1.0, vig, clamp(vignette, 0.0, 1.0));
    return vec4<f32>(col, 1.0) * (alpha * 0.75);
}
