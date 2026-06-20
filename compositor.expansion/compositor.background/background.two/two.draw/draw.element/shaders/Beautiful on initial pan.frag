precision highp float;

// ============================================================================
// [1] UNIFORMS
// ============================================================================
uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform vec2  pan_velocity;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// ============================================================================
// [2] CONSTANTS
// ============================================================================
#define MAX_STEPS 96
#define MAX_DIST 80.0
#define SURF_DIST 0.0015
#define PI 3.14159265359
#define TWO_PI 6.28318530718
#define HALF_PI 1.57079632679
#define EPSILON 0.0001
#define IOR_ICE 1.309
#define FOG_DENSITY 0.055

// -----------------------------------------------------------------------------
// Visual tuning palette
// -----------------------------------------------------------------------------
// Space colors — heavily darkened compared to v2 so the scene reads as DEEP
// space, not twilight. Highlights come from the nebula/stars contrast, not
// from a lifted baseline.
#define DEEP_SPACE       vec3(0.005, 0.008, 0.018)
#define MID_SPACE        vec3(0.020, 0.035, 0.065)
#define HORIZON_TINT     vec3(0.090, 0.135, 0.205)

// Nebula — kept saturated against the dark backdrop
#define NEBULA_A         vec3(0.18, 0.42, 0.65)
#define NEBULA_B         vec3(0.30, 0.18, 0.50)
#define NEBULA_HOT       vec3(0.55, 0.78, 0.95)
#define GALAXY_DUST      vec3(0.32, 0.40, 0.58)
#define GALAXY_CORE      vec3(0.85, 0.90, 1.00)

// Ice & atmospheric tones
#define FOG_COLOR        vec3(0.42, 0.52, 0.68)
#define ICE_ALBEDO       vec3(0.55, 0.72, 0.88)
#define ICE_DEEP         vec3(0.08, 0.22, 0.40)
#define ICE_RIM          vec3(0.75, 0.88, 1.00)

// Stars
#define STAR_WARM        vec3(1.00, 0.88, 0.72)
#define STAR_COOL        vec3(0.72, 0.85, 1.00)
#define STAR_HOT         vec3(0.95, 0.85, 1.00)

// Central calm zone — pulls toward dark slate now, not bright fog
#define CALM_RADIUS      0.42
#define CALM_STRENGTH    0.40
#define CALM_TARGET      vec3(0.08, 0.11, 0.17)

// Galaxy disc — diagonal band of dust crossing the scene
#define GALAXY_ANGLE     0.42

// ============================================================================
// [3] MATRIX MATH
// ============================================================================
mat2 rot2D(float a) {
    float s = sin(a); float c = cos(a);
    return mat2(c, -s, s, c);
}

mat3 rotX(float a) {
    float s = sin(a); float c = cos(a);
    return mat3(1.0, 0.0, 0.0,  0.0, c, -s,  0.0, s, c);
}

mat3 rotY(float a) {
    float s = sin(a); float c = cos(a);
    return mat3(c, 0.0, s,  0.0, 1.0, 0.0,  -s, 0.0, c);
}

mat3 rotZ(float a) {
    float s = sin(a); float c = cos(a);
    return mat3(c, -s, 0.0,  s, c, 0.0,  0.0, 0.0, 1.0);
}

// ============================================================================
// [4] HASHING
// ============================================================================
float hash11(float p) {
    p = fract(p * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float hash13(vec3 p3) {
    p3 = fract(p3 * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec2 hash22(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

vec3 hash33(vec3 p) {
    p = vec3(dot(p, vec3(127.1, 311.7, 74.7)),
             dot(p, vec3(269.5, 183.3, 246.1)),
             dot(p, vec3(113.5, 271.9, 124.6)));
    return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

// ============================================================================
// [5] NOISE LIBRARY
// ============================================================================
float vnoise2D(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = hash12(i);
    float b = hash12(i + vec2(1.0, 0.0));
    float c = hash12(i + vec2(0.0, 1.0));
    float d = hash12(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float vnoise3D(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    vec3 u = f * f * (3.0 - 2.0 * f);
    float a = hash13(i + vec3(0,0,0));
    float b = hash13(i + vec3(1,0,0));
    float c = hash13(i + vec3(0,1,0));
    float d = hash13(i + vec3(1,1,0));
    float e = hash13(i + vec3(0,0,1));
    float g = hash13(i + vec3(1,0,1));
    float h = hash13(i + vec3(0,1,1));
    float j = hash13(i + vec3(1,1,1));
    return mix(
        mix(mix(a,b,u.x), mix(c,d,u.x), u.y),
        mix(mix(e,g,u.x), mix(h,j,u.x), u.y),
        u.z);
}

vec3 voronoi2D(vec2 x) {
    vec2 n = floor(x);
    vec2 f = fract(x);
    float md = 8.0;
    vec2 mr;
    float mh = 0.0;
    for (int j = -1; j <= 1; j++) {
        for (int i = -1; i <= 1; i++) {
            vec2 g = vec2(float(i), float(j));
            vec2 o = hash22(n + g);
            vec2 r = g + o - f;
            float d = dot(r, r);
            if (d < md) { md = d; mr = r; mh = hash12(n + g); }
        }
    }
    return vec3(sqrt(md), mh, mr.x + mr.y);
}

vec2 voronoi3D(vec3 x) {
    vec3 n = floor(x);
    vec3 f = fract(x);
    float f1 = 8.0;
    float f2 = 8.0;
    for (int k = -1; k <= 1; k++) {
        for (int j = -1; j <= 1; j++) {
            for (int i = -1; i <= 1; i++) {
                vec3 g = vec3(float(i), float(j), float(k));
                vec3 o = hash33(n + g) * 0.5 + 0.5;
                vec3 r = g + o - f;
                float d = dot(r, r);
                if (d < f1) { f2 = f1; f1 = d; }
                else if (d < f2) { f2 = d; }
            }
        }
    }
    return vec2(sqrt(f1), sqrt(f2));
}

// ============================================================================
// [6] FBM VARIANTS
// ============================================================================
float fbm2D(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
    for (int i = 0; i < 6; i++) {
        v += a * vnoise2D(p);
        p = m * p;
        a *= 0.5;
    }
    return v;
}

float fbm2D_4(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
    for (int i = 0; i < 4; i++) {
        v += a * vnoise2D(p);
        p = m * p;
        a *= 0.5;
    }
    return v;
}

float fbm3D(vec3 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        v += a * vnoise3D(p);
        p *= 2.02;
        a *= 0.5;
    }
    return v;
}

float ridge3D(vec3 p) {
    float v = 0.0;
    float a = 0.5;
    float w = 1.0;
    for (int i = 0; i < 4; i++) {
        float n = 1.0 - abs(vnoise3D(p) * 2.0 - 1.0);
        n *= n;
        v += a * n * w;
        w = clamp(n * 2.0, 0.0, 1.0);
        p *= 2.13;
        a *= 0.5;
    }
    return v;
}

float domainWarp2D(vec2 p, float t) {
    vec2 q = vec2(fbm2D(p), fbm2D(p + vec2(5.2, 1.3)));
    vec2 r = vec2(fbm2D(p + 3.0 * q + vec2(1.7, 9.2) + t),
                  fbm2D(p + 3.0 * q + vec2(8.3, 2.8) - t));
    return fbm2D(p + 3.0 * r);
}

float domainWarp3D(vec3 p, float t) {
    vec3 q = vec3(
        fbm3D(p),
        fbm3D(p + vec3(5.2, 1.3, 2.8)),
        fbm3D(p + vec3(1.7, 9.2, 4.1))
    );
    vec3 r = vec3(
        fbm3D(p + 3.0 * q + vec3(1.7, 9.2, 3.4) + t),
        fbm3D(p + 3.0 * q + vec3(8.3, 2.8, 1.2) - t),
        fbm3D(p + 3.0 * q + vec3(3.1, 4.5, 6.7))
    );
    return fbm3D(p + 3.0 * r);
}

// ============================================================================
// [7] SDF PRIMITIVES
// ============================================================================
float sdSphere(vec3 p, float s) { return length(p) - s; }

float sdBox(vec3 p, vec3 b) {
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}

float sdOctahedron(vec3 p, float s) {
    p = abs(p);
    float m = p.x + p.y + p.z - s;
    vec3 q;
    if (3.0 * p.x < m) q = p.xyz;
    else if (3.0 * p.y < m) q = p.yzx;
    else if (3.0 * p.z < m) q = p.zxy;
    else return m * 0.57735027;
    float k = clamp(0.5 * (q.z - q.y + s), 0.0, s);
    return length(vec3(q.x, q.y - s + k, q.z - k));
}

float sdHexPrism(vec3 p, vec2 h) {
    const vec3 k = vec3(-0.8660254, 0.5, 0.57735);
    p = abs(p);
    p.xy -= 2.0 * min(dot(k.xy, p.xy), 0.0) * k.xy;
    vec2 d = vec2(
        length(p.xy - vec2(clamp(p.x, -k.z * h.x, k.z * h.x), h.x)) * sign(p.y - h.x),
        p.z - h.y);
    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0));
}

float sdHexBipyramid(vec3 p, float h, float r) {
    p = abs(p);
    const vec2 k = vec2(0.8660254, 0.5);
    p.xz = vec2(p.x * k.x + p.z * k.y, p.z);
    float hexD = max(p.x, p.z * 1.1547) - r;
    float pyrD = p.y - h * (1.0 - max(p.x, p.z) / r);
    return max(hexD, pyrD * 0.6);
}

float sdCappedCone(vec3 p, float h, float r1, float r2) {
    vec2 q = vec2(length(p.xz), p.y);
    vec2 k1 = vec2(r2, h);
    vec2 k2 = vec2(r2 - r1, 2.0 * h);
    vec2 ca = vec2(q.x - min(q.x, (q.y < 0.0) ? r1 : r2), abs(q.y) - h);
    vec2 cb = q - k1 + k2 * clamp(dot(k1 - q, k2) / dot(k2, k2), 0.0, 1.0);
    float s = (cb.x < 0.0 && ca.y < 0.0) ? -1.0 : 1.0;
    return s * sqrt(min(dot(ca, ca), dot(cb, cb)));
}

float sdRoundBox(vec3 p, vec3 b, float r) {
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

// ============================================================================
// [8] SDF OPS
// ============================================================================
float opUnion(float d1, float d2) { return min(d1, d2); }
float opSubtract(float d1, float d2) { return max(-d1, d2); }
float opIntersect(float d1, float d2) { return max(d1, d2); }

float opSmoothUnion(float d1, float d2, float k) {
    float h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

float opSmoothSubtract(float d1, float d2, float k) {
    float h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);
    return mix(d2, -d1, h) + k * h * (1.0 - h);
}

vec3 opTwist(vec3 p, float k) {
    float c = cos(k * p.y);
    float s = sin(k * p.y);
    mat2 m = mat2(c, -s, s, c);
    return vec3(m * p.xz, p.y);
}

// ============================================================================
// [9] PBR LIGHTING
// ============================================================================
struct Material {
    vec3 albedo;
    float roughness;
    float metallic;
    float transmission;
};

struct Light {
    vec3 direction;
    vec3 color;
    float intensity;
};

vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

float distributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    return a2 / max(denom, 1e-7);
}

float geometrySchlickGGX(float NdotV, float roughness) {
    float r = roughness + 1.0;
    float k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

float geometrySmith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    return geometrySchlickGGX(NdotV, roughness) * geometrySchlickGGX(NdotL, roughness);
}

vec3 calculatePBR(vec3 N, vec3 V, vec3 L, Material m, Light light) {
    vec3 H = normalize(V + L);
    vec3 F0 = mix(vec3(0.04), m.albedo, m.metallic);
    float NDF = distributionGGX(N, H, m.roughness);
    float G = geometrySmith(N, V, L, m.roughness);
    vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);
    vec3 spec = (NDF * G * F) / max(4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0), 1e-4);
    vec3 kD = (vec3(1.0) - F) * (1.0 - m.metallic);
    float NdotL = max(dot(N, L), 0.0);
    return (kD * m.albedo / PI + spec) * light.color * light.intensity * NdotL;
}

// ============================================================================
// [10] FOREGROUND SCENE — raymarched ice crystals
// ----------------------------------------------------------------------------
// Material id encoded in the SDF return:
//   1.0 = standard distant ice (gets fog + DOF blur)
//   2.0 = foreground anchor (stays sharp, stronger rim) — gives the eye
//         a reliable depth reference and sells parallax
// All crystals along bottom/edges; central 60% of frame intentionally empty
// for UI on top.
// ============================================================================
struct MapResult { float d; float mat; };

MapResult mapScene(vec3 p) {
    p.y += sin(u_time * 0.1) * 0.04;

    MapResult res;
    res.d = MAX_DIST;
    res.mat = 1.0;

    // --- Cluster A: left edge anchor (SHARP foreground) ---
    {
        vec3 lp = p - vec3(-2.8, -1.0, 0.5);
        lp = rotY(u_time * 0.06 + 1.3) * rotZ(0.4) * lp;
        float s = sdHexBipyramid(lp, 1.7, 0.42);
        s -= ridge3D(lp * 4.0) * 0.045;
        if (s < res.d) { res.d = s; res.mat = 2.0; }
    }

    // --- Cluster B: left mid ---
    {
        vec3 lp = p - vec3(-2.0, -0.3, 1.8);
        lp = rotZ(u_time * 0.05 - 0.6) * lp;
        float s = sdOctahedron(lp, 0.55);
        s -= ridge3D(lp * 5.0) * 0.05;
        if (s < res.d) { res.d = opSmoothUnion(res.d, s, 0.15); res.mat = 1.0; }
    }

    // --- Cluster C: left lower small ---
    {
        vec3 lp = p - vec3(-1.4, -1.4, 2.4);
        lp = rotX(0.3) * rotY(u_time * 0.12 + 2.1) * lp;
        float s = sdHexBipyramid(lp, 0.9, 0.22);
        s -= ridge3D(lp * 6.0) * 0.03;
        if (s < res.d) { res.d = s; res.mat = 1.0; }
    }

    // --- Cluster D: bottom center-left chunky ---
    {
        vec3 lp = p - vec3(-0.6, -1.6, 3.0);
        lp = rotY(u_time * 0.06) * lp;
        float s = sdHexPrism(lp, vec2(0.4, 0.8));
        float capTop = sdCappedCone(lp - vec3(0.0, 0.8, 0.0), 0.35, 0.4, 0.05);
        s = opSmoothUnion(s, capTop, 0.08);
        s -= ridge3D(lp * 5.0) * 0.04;
        if (s < res.d) { res.d = opSmoothUnion(res.d, s, 0.1); res.mat = 1.0; }
    }

    // --- Cluster E: bottom small floating ---
    {
        vec3 lp = p - vec3(0.1, -1.8, 2.5);
        lp = rotZ(u_time * 0.15 + 0.8) * lp;
        float s = sdOctahedron(lp, 0.32);
        if (s < res.d) { res.d = s; res.mat = 1.0; }
    }

    // --- Cluster F: bottom right pair ---
    {
        vec3 lp = p - vec3(1.2, -1.5, 2.2);
        lp = rotY(u_time * 0.07 - 0.4) * rotX(0.2) * lp;
        float s = sdHexBipyramid(lp, 1.1, 0.28);
        s -= ridge3D(lp * 5.5) * 0.04;
        if (s < res.d) { res.d = s; res.mat = 1.0; }
    }

    // --- Cluster G: right edge anchor (SHARP foreground) ---
    {
        vec3 lp = p - vec3(2.5, -0.5, 0.6);
        lp = rotZ(u_time * 0.04 + 0.3) * lp;
        float s = sdHexBipyramid(lp, 1.9, 0.45);
        s -= ridge3D(lp * 4.5) * 0.05;
        if (s < res.d) { res.d = s; res.mat = 2.0; }
    }

    // --- Cluster H: distant scattered (far Z, heavy DOF) ---
    for (int i = 0; i < 3; i++) {
        float fi = float(i);
        vec3 pos = vec3(
            -1.5 + fi * 1.3,
            -0.8 + sin(fi * 2.1) * 0.4,
            5.0 + fi * 0.6
        );
        vec3 lp = p - pos;
        lp = rotY(u_time * 0.1 + fi * 2.4) * rotZ(fi * 1.7) * lp;
        float s = sdOctahedron(lp, 0.28 + fi * 0.06);
        if (s < res.d) { res.d = s; res.mat = 1.0; }
    }

    return res;
}

vec3 calcNormal(vec3 p) {
    vec2 e = vec2(EPSILON, 0.0);
    return normalize(vec3(
        mapScene(p + e.xyy).d - mapScene(p - e.xyy).d,
        mapScene(p + e.yxy).d - mapScene(p - e.yxy).d,
        mapScene(p + e.yyx).d - mapScene(p - e.yyx).d
    ));
}

float softShadow(vec3 ro, vec3 rd, float mint, float maxt, float k) {
    float res = 1.0;
    float t = mint;
    for (int i = 0; i < 24; i++) {
        float h = mapScene(ro + rd * t).d;
        res = min(res, k * h / t);
        t += clamp(h, 0.03, 0.3);
        if (res < 0.005 || t > maxt) break;
    }
    return clamp(res, 0.0, 1.0);
}

float calcAO(vec3 p, vec3 n) {
    float occ = 0.0;
    float sca = 1.0;
    for (int i = 0; i < 5; i++) {
        float h = 0.02 + 0.15 * float(i) / 4.0;
        float d = mapScene(p + h * n).d;
        occ += (h - d) * sca;
        sca *= 0.92;
    }
    return clamp(1.0 - 2.5 * occ, 0.0, 1.0);
}

// ============================================================================
// [11] PLANET RENDERER
// ============================================================================
struct PlanetResult {
    vec3 color;
    float alpha;
    vec3 halo;
};

PlanetResult renderPlanet(
    vec2 uv,
    vec2 center,
    float radius,
    vec3 baseColor,
    vec3 iceColor,
    vec2 lightDir,
    float surfaceSeed,
    float blurAmount
) {
    PlanetResult res;
    res.color = vec3(0.0);
    res.alpha = 0.0;
    res.halo = vec3(0.0);

    vec2 p = uv - center;
    float d = length(p);

    float edgeWidth = 0.003 + blurAmount * 0.045;
    float mask = smoothstep(radius + edgeWidth, radius - edgeWidth, d);

    float haloFalloff = 9.0 - blurAmount * 5.0;
    float halo = exp(-(d - radius) * haloFalloff);
    halo = max(halo, 0.0);
    float haloLit = smoothstep(-radius * 0.5, radius * 1.5, dot(p, lightDir));
    // Dimmer halo for dark space — atmospheric scatter against a black void
    res.halo = iceColor * halo * haloLit * (0.35 + 0.25 * (1.0 - blurAmount));

    if (mask <= 0.001) return res;

    float z = sqrt(max(0.0, radius * radius - d * d));
    vec3 normal = normalize(vec3(p.x, p.y, z));
    vec3 L = normalize(vec3(lightDir.x, lightDir.y, 0.6));

    vec2 sp = normal.xy * 1.8 + vec2(surfaceSeed * 11.3, surfaceSeed * 7.7);
    sp += vec2(u_time * 0.005, 0.0);

    float continents = domainWarp2D(sp * 2.5, u_time * 0.01);
    vec3 v3 = voronoi2D(sp * 4.0);
    float cracks = smoothstep(0.0, 0.15, v3.x);
    float pole = pow(abs(normal.y), 1.5);

    vec3 surface = mix(baseColor, iceColor, smoothstep(0.4, 0.7, continents));
    surface = mix(iceColor * 1.05, surface, cracks);
    surface = mix(surface, vec3(0.92, 0.95, 1.0), pole * 0.55);

    // Darker unlit side — sells "space lighting"
    float diff = max(dot(normal, L), 0.0);
    float wrapDiff = max(0.0, (dot(normal, L) + 0.2) / 1.2);
    float NdotV = max(normal.z, 0.0);
    float rim = pow(1.0 - NdotV, 3.0);
    vec3 H = normalize(L + vec3(0.0, 0.0, 1.0));
    float spec = pow(max(dot(normal, H), 0.0), 32.0) * (1.0 - cracks) * 0.4;

    vec3 final = surface * (wrapDiff * 0.95 + 0.04);
    final += iceColor * rim * diff * 0.7;
    final += vec3(1.0) * spec;

    // Blur mixes toward dark space, not bright fog
    final = mix(final, MID_SPACE * 2.0, blurAmount * 0.6);

    res.color = final;
    res.alpha = mask;
    return res;
}

// ============================================================================
// [12] STARFIELD — denser, with greater dynamic range
// ----------------------------------------------------------------------------
// 5 parallax layers, with dim background dust stars + bright distinct stars.
// Slowest layer = nearly stationary (deep space), fastest = moves visibly.
// Differential parallax is the main depth cue.
// ============================================================================
vec3 renderStars(vec2 uv, vec2 pan) {
    vec3 col = vec3(0.0);

    for (float i = 0.0; i < 5.0; i += 1.0) {
        float scale = 30.0 + i * 22.0;
        float parallax = 0.05 + i * 0.20;  // very slow far layers
        vec2 sp = uv * scale + pan * parallax;
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i * 12.34);

        // Dim background dust stars — dense, very low brightness
        if (h > 0.82) {
            float r = length(fp);
            float dim = exp(-r * 55.0) * (h - 0.82) * 1.0;
            float phase = h * 137.5 + u_time * (0.8 + h * 2.0);
            float twinkle = 0.7 + 0.3 * sin(phase);
            vec3 dimCol = mix(STAR_COOL, STAR_WARM, fract(h * 33.3)) * 0.55;
            col += dimCol * dim * twinkle;
        }

        // Bright distinct stars (rare)
        if (h > 0.96) {
            float brightness = (h - 0.96) * 30.0;
            float r = length(fp);
            float phase = h * 217.5 + u_time * (1.2 + h * 2.8);
            float twinkle = 0.55 + 0.45 * sin(phase);

            float core = exp(-r * (35.0 - brightness * 6.0));

            float glint = 0.0;
            if (brightness > 0.3) {
                float gx = exp(-abs(fp.y) * 90.0) * exp(-abs(fp.x) * 6.0);
                float gy = exp(-abs(fp.x) * 90.0) * exp(-abs(fp.y) * 6.0);
                glint = (gx + gy) * 0.5 * (brightness - 0.3);
            }

            // Color: mostly cool, some warm, rare hot pink-white
            float colorPick = fract(h * 71.3);
            vec3 starCol;
            if (colorPick > 0.9) starCol = STAR_HOT;
            else if (colorPick > 0.7) starCol = STAR_WARM;
            else starCol = STAR_COOL;

            col += starCol * (core + glint) * twinkle * brightness * 1.1;
        }
    }
    return col;
}

// ============================================================================
// [13] DISTANT GALAXIES & GALACTIC DUST BAND
// ----------------------------------------------------------------------------
// The galaxy band runs diagonally and is what tells the eye "this is inside
// a galaxy" — a faint Milky-Way-edge feel. Plus two tiny distant spirals.
// ============================================================================
vec3 renderGalaxyBand(vec2 uv, vec2 pan) {
    vec2 g = rot2D(GALAXY_ANGLE) * uv;
    g += pan * 0.05;  // very slow parallax — distant

    float bandY = g.y - 0.15;
    float bandWidth = 0.25;
    float bandMask = exp(-pow(bandY / bandWidth, 2.0));

    float dust = fbm2D(g * vec2(2.0, 8.0) + vec2(u_time * 0.003, 0.0));
    float lanes = smoothstep(0.55, 0.75, dust);
    float glow = bandMask * (0.4 + 0.6 * fbm2D(g * vec2(3.0, 12.0)));

    vec3 col = GALAXY_DUST * glow * 0.25;
    col *= 1.0 - lanes * 0.6 * bandMask;
    return col;
}

vec3 renderDistantGalaxies(vec2 uv, vec2 pan) {
    vec3 col = vec3(0.0);

    // Galaxy 1 — upper right area
    {
        vec2 center = vec2(0.55, 0.42) - pan * 0.08;
        vec2 p = uv - center;
        p = rot2D(0.6) * p;
        p.x *= 0.5;
        float r = length(p);
        float glow = exp(-r * 18.0) * 0.6;
        float core = exp(-r * 60.0) * 1.2;
        col += (GALAXY_DUST * glow + GALAXY_CORE * core) * 0.35;
    }

    // Galaxy 2 — upper left
    {
        vec2 center = vec2(-0.45, 0.50) - pan * 0.06;
        vec2 p = uv - center;
        p = rot2D(-0.3) * p;
        p.x *= 0.6;
        float r = length(p);
        float glow = exp(-r * 25.0) * 0.4;
        float core = exp(-r * 80.0) * 0.9;
        col += (NEBULA_A * glow + GALAXY_CORE * core) * 0.25;
    }

    return col;
}

// ============================================================================
// [14] NEBULA
// ----------------------------------------------------------------------------
// Higher contrast against the darker sky. Two-color blend with hot cores.
// ============================================================================
vec3 renderNebula(vec2 uv, vec2 pan, vec2 flow) {
    vec3 warpPos = vec3(uv * 1.3 + pan * 0.35 - flow * 0.5, u_time * 0.012);
    float density = domainWarp3D(warpPos, u_time * 0.006);
    density = smoothstep(0.32, 0.85, density);

    float detail = fbm3D(warpPos * 3.5 + vec3(2.3, 1.7, 0.0));
    density *= 0.7 + 0.3 * detail;

    float colorMix = fbm3D(warpPos * 2.0 + vec3(7.1, 3.4, 0.0));
    vec3 col = mix(NEBULA_A, NEBULA_B, colorMix);

    float cores = smoothstep(0.75, 0.95, density);
    col = mix(col, NEBULA_HOT, cores * 0.7);

    return col * density;
}

// ============================================================================
// [15] DUST MOTES — intermediate-depth drifting particles
// ----------------------------------------------------------------------------
// 4th particle tier (between stars and snow in depth): tiny bright points,
// slow motion, low parallax, pulse-twinkle.
// ============================================================================
float dustMotes(vec2 uv, vec2 pan, vec2 flow) {
    float total = 0.0;
    for (float i = 0.0; i < 3.0; i += 1.0) {
        float scale = 35.0 + i * 20.0;
        vec2 sp = uv * scale;
        sp += vec2(u_time * (0.04 + i * 0.02), u_time * (0.02 + i * 0.015));
        sp += pan * (0.2 + i * 0.1) + flow * (0.3 + i * 0.15);

        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i * 17.7);

        if (h > 0.92) {
            float r = length(fp);
            float phase = h * 100.0 + u_time * 1.5;
            float pulse = 0.6 + 0.4 * sin(phase);
            total += exp(-r * 70.0) * (h - 0.92) * 8.0 * pulse;
        }
    }
    return total;
}

// ============================================================================
// [16] CLOUD BANDS — darker
// ============================================================================
float cloudBand(vec2 uv, float yCenter, float yWidth, float scale, float scrollSpeed, float seed) {
    float bandMask = exp(-pow((uv.y - yCenter) / yWidth, 2.0));
    vec2 cp = uv * scale + vec2(u_time * scrollSpeed + seed, seed * 0.7);
    float density = domainWarp2D(cp, u_time * 0.02);
    density = smoothstep(0.35, 0.75, density);
    density *= 0.7 + 0.3 * fbm2D(cp * 3.0);
    return density * bandMask;
}

// ============================================================================
// [17] LIGHT SHAFTS — directional volumetric scatter
// ----------------------------------------------------------------------------
// Cheap god-rays: radial falloff from a "light origin" in the upper-right,
// modulated by streak noise perpendicular to the light direction.
// ============================================================================
float lightShafts(vec2 uv, vec2 flow) {
    vec2 lightDir = normalize(vec2(0.6, 0.4));
    float along = dot(uv, lightDir);

    vec2 lightOrigin = vec2(0.8, 0.6);
    float distFromSource = length(uv - lightOrigin);
    float radial = exp(-distFromSource * 1.5);

    vec2 perp = vec2(-lightDir.y, lightDir.x);
    float streakCoord = dot(uv, perp) * 4.0;
    float streakNoise = fbm2D_4(vec2(streakCoord, along * 2.0) + flow * 0.5 + vec2(u_time * 0.03, 0.0));
    float streaks = smoothstep(0.4, 0.7, streakNoise);

    return radial * streaks * 0.5;
}

// ============================================================================
// [18] SNOW PARTICLES
// ============================================================================
float snowLayer(vec2 uv, float scale, float speed, float seed, vec2 drift, vec2 motionBlur, float threshold) {
    vec2 sp = uv * scale;
    sp.y += u_time * speed;
    sp += drift;
    sp.x += sin(sp.y * 1.7 + seed * 6.0 + u_time * 0.5) * 0.3;
    sp.x += sin(sp.y * 0.4 + seed * 2.0) * 0.6;

    vec2 id = floor(sp);
    vec2 fp = fract(sp) - 0.5;
    float h = hash12(id + seed);
    if (h < threshold) return 0.0;

    float mbLen = length(motionBlur) / max(scale, 1.0);
    if (mbLen > 0.001) {
        vec2 dir = normalize(motionBlur);
        float par = dot(fp, dir);
        float per = dot(fp, vec2(-dir.y, dir.x));
        par /= (1.0 + mbLen * 6.0);
        fp = dir * par + vec2(-dir.y, dir.x) * per;
    }

    float r = length(fp);
    float size = 0.04 + (h - threshold) * 0.7;
    float core = smoothstep(size, 0.0, r);
    float halo = smoothstep(size * 2.5, size * 0.8, r) * 0.2;
    return core + halo;
}

vec3 renderSnow(vec2 uv, vec2 pan, vec2 flow, vec2 vel) {
    float total = 0.0;
    total += snowLayer(uv, 22.0, 0.08, 3.1,  pan * 0.3 + flow * 0.4, vel * 0.2, 0.86) * 0.35;
    total += snowLayer(uv, 17.0, 0.13, 5.7,  pan * 0.4 + flow * 0.6, vel * 0.3, 0.85) * 0.5;
    total += snowLayer(uv, 12.0, 0.22, 7.7,  pan * 0.6 + flow * 0.9, vel * 0.5, 0.82) * 0.65;
    total += snowLayer(uv,  9.0, 0.30, 11.3, pan * 0.8 + flow * 1.2, vel * 0.7, 0.80) * 0.8;
    total += snowLayer(uv,  6.0, 0.42, 13.9, pan * 1.1 + flow * 1.6, vel * 0.9, 0.78) * 0.9;
    total += snowLayer(uv,  3.5, 0.60, 21.3, pan * 1.5 + flow * 2.2, vel * 1.2, 0.75) * 0.65;
    return vec3(0.92, 0.96, 1.0) * total;
}

// ============================================================================
// [19] POST-PROCESS HELPERS
// ============================================================================
vec3 reinhardTonemap(vec3 c) {
    return c / (1.0 + c);
}

// Dark filmic — crushes blacks deeper than Reinhard while preserving
// nebula/star highlights.
vec3 darkFilmic(vec3 x) {
    float a = 1.8;
    float b = 0.05;
    float c = 1.8;
    float d = 0.55;
    float e = 0.08;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

vec3 desaturate(vec3 c, float amount) {
    float lum = dot(c, vec3(0.299, 0.587, 0.114));
    return mix(c, vec3(lum), amount);
}

float vignette(vec2 fragUv, float intensity, float smoothness) {
    float dist = distance(fragUv, vec2(0.5));
    return smoothstep(intensity, intensity - smoothness, dist);
}

float fogFactor(float t, float density) {
    return 1.0 - exp(-t * density);
}

// ============================================================================
// [20] MAIN RENDER
// ============================================================================
void main() {
    vec2 fragUv = gl_FragCoord.xy / u_resolution.xy;
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution.xy) / u_resolution.y;

    float z = clamp(u_zoom, 0.4, 3.0);
    uv /= z;

    vec2 pan = vec2(u_pan.x, -u_pan.y) * 0.0008;
    vec2 flow = u_flow_offset * 0.0006;
    vec2 vel = pan_velocity * 0.0006;

    float vlen = length(vel);
    if (vlen > 0.18) vel *= 0.18 / vlen;

    // ========================================================================
    // PASS 1 — Deep space gradient (darker than v2)
    // ========================================================================
    float vGrad = clamp(fragUv.y, 0.0, 1.0);
    vec3 col = mix(HORIZON_TINT, MID_SPACE, smoothstep(0.0, 0.45, vGrad));
    col = mix(col, DEEP_SPACE, smoothstep(0.45, 0.95, vGrad));
    col += vec3(0.005, 0.008, 0.012) * (1.0 - fragUv.x) * (1.0 - vGrad);

    // ========================================================================
    // PASS 2 — Distant galaxies
    // ========================================================================
    col += renderDistantGalaxies(uv, pan);

    // ========================================================================
    // PASS 3 — Galactic dust band
    // ========================================================================
    col += renderGalaxyBand(uv, pan);

    // ========================================================================
    // PASS 4 — Nebula
    // ========================================================================
    col += renderNebula(uv, pan, flow) * 0.65;

    // ========================================================================
    // PASS 5 — Starfield
    // ========================================================================
    col += renderStars(uv, pan);

    // ========================================================================
    // PASS 6 — Light shafts
    // ========================================================================
    float shafts = lightShafts(uv, flow);
    col += vec3(0.45, 0.55, 0.75) * shafts * 0.15;

    // ========================================================================
    // PASS 7 — Planets
    // ========================================================================
    vec2 lightDir2D = normalize(vec2(0.7, 0.55));

    {
        PlanetResult pl = renderPlanet(
            uv, vec2(0.85, 0.45) - pan * 0.15, 0.08,
            vec3(0.10, 0.18, 0.30), vec3(0.50, 0.68, 0.85),
            lightDir2D, 3.7, 0.15
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    {
        PlanetResult pl = renderPlanet(
            uv, vec2(-0.25, 0.55) - pan * 0.25, 0.07,
            vec3(0.12, 0.20, 0.32), vec3(0.58, 0.72, 0.88),
            lightDir2D, 7.1, 0.1
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    {
        PlanetResult pl = renderPlanet(
            uv, vec2(-0.65, 0.20) - pan * 0.35, 0.28,
            vec3(0.08, 0.14, 0.26), vec3(0.55, 0.70, 0.85),
            lightDir2D, 11.3, 0.30
        );
        col += pl.halo * 0.6;
        col = mix(col, pl.color, pl.alpha * 0.90);
    }

    {
        PlanetResult pl = renderPlanet(
            uv, vec2(0.05, -0.05) - pan * 0.5, 0.13,
            vec3(0.12, 0.20, 0.32), vec3(0.62, 0.78, 0.92),
            lightDir2D, 17.9, 0.20
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    {
        PlanetResult pl = renderPlanet(
            uv, vec2(0.75, -0.10) - pan * 0.6, 0.42,
            vec3(0.14, 0.22, 0.36), vec3(0.70, 0.85, 0.98),
            lightDir2D, 23.4, 0.10
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    // ========================================================================
    // PASS 8 — Cloud bands (darker mix targets)
    // ========================================================================
    float midClouds = cloudBand(uv + pan * 0.6 + flow * 1.2, -0.05, 0.55, 1.8, 0.04, 4.2);
    col = mix(col, vec3(0.32, 0.40, 0.55), midClouds * 0.45);

    float lowerClouds = cloudBand(uv + pan * 0.8 + flow * 1.6, -0.45, 0.4, 2.5, 0.06, 9.1);
    col = mix(col, vec3(0.38, 0.46, 0.60), lowerClouds * 0.55);

    float upperHaze = cloudBand(uv + pan * 0.4 + flow * 0.7, 0.45, 0.35, 1.2, 0.025, 13.7);
    col = mix(col, vec3(0.18, 0.25, 0.40), upperHaze * 0.20);

    // ========================================================================
    // PASS 9 — Dust motes (4th particle tier)
    // ========================================================================
    float motes = dustMotes(uv, pan, flow);
    col += vec3(0.78, 0.85, 0.98) * motes * 0.7;

    // ========================================================================
    // PASS 10 — Raymarched ice crystals
    // ========================================================================
    vec3 ro = vec3(pan.x * 0.4, -pan.y * 0.4 + 0.1, -4.0);
    vec3 rd = normalize(vec3(uv * 1.2, 1.0));
    rd = rotX(-0.05) * rd;

    float t = 0.5;
    bool hit = false;
    MapResult hitRes;
    hitRes.d = 0.0;
    hitRes.mat = 0.0;
    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 p = ro + rd * t;
        MapResult mr = mapScene(p);
        if (mr.d < SURF_DIST) { hit = true; hitRes = mr; break; }
        t += mr.d * 0.9;
        if (t > MAX_DIST) break;
    }

    if (hit) {
        vec3 p = ro + rd * t;
        vec3 n = calcNormal(p);
        vec3 v = normalize(ro - p);

        Light light;
        light.direction = normalize(vec3(0.7, 0.6, -0.5));
        light.color = vec3(0.88, 0.92, 1.0);
        // Anchor crystals (mat 2.0) get stronger lighting to compete with the
        // dark backdrop — they should read as solid foreground objects.
        light.intensity = (hitRes.mat > 1.5) ? 2.6 : 1.9;

        Material ice;
        ice.albedo = ICE_ALBEDO;
        ice.roughness = (hitRes.mat > 1.5) ? 0.06 : 0.10;
        ice.metallic = 0.05;
        ice.transmission = 0.7;

        vec3 pbr = calculatePBR(n, v, light.direction, ice, light);
        float ao = calcAO(p, n);
        float sh = softShadow(p, light.direction, 0.04, 4.0, 12.0);

        float sss = pow(max(0.0, dot(v, -light.direction)), 4.0) * 0.5;
        vec3 sssCol = ICE_DEEP * sss * 1.4;

        vec3 refRay = refract(rd, n, 1.0 / IOR_ICE);
        vec2 refUv = uv + refRay.xy * 0.3;
        float refFog = domainWarp2D(refUv * 2.5 + flow, u_time * 0.01);
        vec3 refColor = mix(ICE_DEEP, FOG_COLOR * 0.8, smoothstep(0.3, 0.7, refFog));

        vec3 F0 = vec3(0.04);
        vec3 fresnel = fresnelSchlick(max(dot(n, v), 0.0), F0);
        float rim = pow(1.0 - max(dot(n, v), 0.0), 3.0);
        float rimStrength = (hitRes.mat > 1.5) ? 1.3 : 0.7;

        vec3 crystalCol = pbr * sh * ao;
        crystalCol += sssCol * ao;
        crystalCol += refColor * 0.45 * (1.0 - rim);
        crystalCol += ICE_RIM * rim * rimStrength;
        crystalCol = mix(crystalCol, ICE_ALBEDO * 0.55, 0.15);

        // Distance fog blends toward dark mid-space, not bright fog
        float fogAmount = fogFactor(t, FOG_DENSITY);
        vec3 distantFog = mix(FOG_COLOR * 0.55, MID_SPACE * 1.4, 0.6);
        crystalCol = mix(crystalCol, distantFog, fogAmount);

        // DOF — anchors bypass it, standard crystals get focal blur
        float dofBlur;
        if (hitRes.mat > 1.5) {
            dofBlur = 0.0;
        } else {
            float dofFocal = 3.5;
            float dofRange = 6.0;
            dofBlur = smoothstep(0.0, dofRange, abs(t - dofFocal));
        }

        col = mix(crystalCol, col, dofBlur * 0.75);
    }

    // ========================================================================
    // PASS 11 — Snowfall
    // ========================================================================
    col += renderSnow(uv, pan, flow, vel);

    // ========================================================================
    // PASS 12 — Volumetric fog veil (now mixes toward a darker tone)
    // ========================================================================
    vec2 veilUv = uv * 0.8 + pan * 0.5 + flow * 1.0 + vec2(u_time * 0.01, u_time * 0.005);
    float veilDensity = domainWarp2D(veilUv, u_time * 0.015);
    veilDensity = smoothstep(0.2, 0.8, veilDensity);

    float veilVerticalMask = smoothstep(0.0, 0.35, fragUv.y) * smoothstep(1.0, 0.65, fragUv.y);
    veilVerticalMask = max(veilVerticalMask, 0.35);

    float veilAmount = veilDensity * veilVerticalMask * 0.22;
    col = mix(col, FOG_COLOR * 0.7, veilAmount);

    // ========================================================================
    // PASS 13 — Central calm zone — pulls to dark slate, not bright fog
    // ========================================================================
    float distFromCenter = length(fragUv - 0.5);
    float calmMask = 1.0 - smoothstep(CALM_RADIUS - 0.18, CALM_RADIUS + 0.18, distFromCenter);
    col = mix(col, CALM_TARGET, calmMask * CALM_STRENGTH * 0.30);

    // ========================================================================
    // PASS 14 — Final grading
    // ========================================================================
    col = darkFilmic(col);
    col = desaturate(col, 0.15);
    col *= vec3(0.94, 0.98, 1.05);

    // Stronger vignette so corners go quite dark
    float vig = vignette(fragUv, 0.95, 0.55);
    col *= mix(0.55, 1.0, vig);

    float grain = (hash12(gl_FragCoord.xy + fract(u_time)) - 0.5) * 0.012;
    col += grain;

    // Floor blacks just above zero to avoid banding without lifting
    col = max(col, vec3(0.005, 0.008, 0.014));

    col = clamp(col, 0.0, 1.0);

    gl_FragColor = vec4(col, 1.0) * alpha;
}
