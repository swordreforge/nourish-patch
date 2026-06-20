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
#define FOG_DENSITY 0.045

// Visual tuning — collected here so the whole composition can be re-balanced
// without hunting through the file.
#define FOG_COLOR        vec3(0.78, 0.84, 0.92)
#define DEEP_SKY_COLOR   vec3(0.06, 0.10, 0.18)
#define HORIZON_COLOR    vec3(0.55, 0.68, 0.82)
#define NEBULA_A         vec3(0.20, 0.42, 0.58)
#define NEBULA_B         vec3(0.32, 0.28, 0.48)
#define ICE_ALBEDO       vec3(0.62, 0.78, 0.92)
#define ICE_DEEP         vec3(0.10, 0.28, 0.46)
#define STAR_WARM        vec3(1.00, 0.92, 0.78)
#define STAR_COOL        vec3(0.75, 0.88, 1.00)

// Central calm zone — anything inside this radius gets pulled toward fog color
// so UI on top stays readable.
#define CALM_RADIUS 0.42
#define CALM_STRENGTH 0.55

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

// 2D voronoi — F1 distance, plus cell id hash for coloring
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
            if (d < md) {
                md = d;
                mr = r;
                mh = hash12(n + g);
            }
        }
    }
    return vec3(sqrt(md), mh, mr.x + mr.y);
}

// 3D voronoi — F1 + F2 for edge effects (used in ice crack patterns)
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

// Ridged multifractal — for crisp cracks/edges
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

// Domain-warped fbm — gives clouds/nebulae their swirling fluid character
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
// [7] SDF PRIMITIVES — ice crystal building blocks
// ============================================================================
float sdSphere(vec3 p, float s) {
    return length(p) - s;
}

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

// Hexagonal bipyramid — classic ice crystal shape
float sdHexBipyramid(vec3 p, float h, float r) {
    p = abs(p);
    // hex prism cross-section in xz
    const vec2 k = vec2(0.8660254, 0.5);
    p.xz = vec2(p.x * k.x + p.z * k.y, p.z);
    float hexD = max(p.x, p.z * 1.1547) - r;
    // pyramid cap
    float pyrD = p.y - h * (1.0 - max(p.x, p.z) / r);
    return max(hexD, pyrD * 0.6);
}

// Capped cone — useful for shard tips
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
// [8] SDF OPERATIONS
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

// Bend p around its own axis — used to give crystals organic asymmetry
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
// [10] FOREGROUND ICE SCENE (raymarched)
// ----------------------------------------------------------------------------
// Crystals positioned along the bottom edge & corners. Center is left empty
// so UI on top stays readable. Each crystal is a different SDF so the cluster
// reads as heterogeneous debris rather than a tiled pattern.
// ============================================================================

// Forward declaration not needed in GLSL — order matters. mapScene defined first.
float mapScene(vec3 p) {
    // Drift the whole field slowly so the foreground feels alive
    p.y += sin(u_time * 0.1) * 0.05;

    float d = MAX_DIST;

    // We carve up the scene into a handful of hand-placed crystal clusters
    // along the lower border. Hard-coded positions because procedural placement
    // tends to drift crystals into the center and obscure UI.
    //
    // Layout (xy roughly, z negative = toward camera):
    //   left edge: 3 crystals descending
    //   bottom: a cluster of 4
    //   right edge: 2 large
    //   scattered: 3 distant
    //
    // Each crystal: pos, rotation seed, scale, sdf-type id.

    // Cluster A — left edge tall shard
    {
        vec3 lp = p - vec3(-2.4, -1.1, 1.2);
        lp = rotY(u_time * 0.08 + 1.3) * rotZ(0.4) * lp;
        float s = sdHexBipyramid(lp, 1.6, 0.35);
        s -= ridge3D(lp * 4.0) * 0.04;
        d = opUnion(d, s);
    }

    // Cluster B — left mid
    {
        vec3 lp = p - vec3(-2.0, -0.3, 1.8);
        lp = rotZ(u_time * 0.05 - 0.6) * lp;
        float s = sdOctahedron(lp, 0.55);
        s -= ridge3D(lp * 5.0) * 0.05;
        d = opSmoothUnion(d, s, 0.15);
    }

    // Cluster C — left lower small
    {
        vec3 lp = p - vec3(-1.4, -1.4, 2.4);
        lp = rotX(0.3) * rotY(u_time * 0.12 + 2.1) * lp;
        float s = sdHexBipyramid(lp, 0.9, 0.22);
        s -= ridge3D(lp * 6.0) * 0.03;
        d = opUnion(d, s);
    }

    // Cluster D — bottom center-left chunky
    {
        vec3 lp = p - vec3(-0.6, -1.6, 3.0);
        lp = rotY(u_time * 0.06) * lp;
        float s = sdHexPrism(lp, vec2(0.4, 0.8));
        float capTop = sdCappedCone(lp - vec3(0.0, 0.8, 0.0), 0.35, 0.4, 0.05);
        s = opSmoothUnion(s, capTop, 0.08);
        s -= ridge3D(lp * 5.0) * 0.04;
        d = opSmoothUnion(d, s, 0.1);
    }

    // Cluster E — bottom small floating
    {
        vec3 lp = p - vec3(0.1, -1.8, 2.5);
        lp = rotZ(u_time * 0.15 + 0.8) * lp;
        float s = sdOctahedron(lp, 0.32);
        d = opUnion(d, s);
    }

    // Cluster F — bottom right pair
    {
        vec3 lp = p - vec3(1.2, -1.5, 2.2);
        lp = rotY(u_time * 0.07 - 0.4) * rotX(0.2) * lp;
        float s = sdHexBipyramid(lp, 1.1, 0.28);
        s -= ridge3D(lp * 5.5) * 0.04;
        d = opUnion(d, s);
    }

    // Cluster G — right tall
    {
        vec3 lp = p - vec3(2.2, -0.6, 1.6);
        lp = rotZ(u_time * 0.04 + 0.3) * lp;
        float s = sdHexBipyramid(lp, 1.8, 0.38);
        s -= ridge3D(lp * 4.5) * 0.05;
        d = opUnion(d, s);
    }

    // Cluster H — distant scattered (far Z, will get DOF blurred)
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
        d = opUnion(d, s);
    }

    return d;
}

vec3 calcNormal(vec3 p) {
    vec2 e = vec2(EPSILON, 0.0);
    return normalize(vec3(
        mapScene(p + e.xyy) - mapScene(p - e.xyy),
        mapScene(p + e.yxy) - mapScene(p - e.yxy),
        mapScene(p + e.yyx) - mapScene(p - e.yyx)
    ));
}

float softShadow(vec3 ro, vec3 rd, float mint, float maxt, float k) {
    float res = 1.0;
    float t = mint;
    for (int i = 0; i < 24; i++) {
        float h = mapScene(ro + rd * t);
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
        float d = mapScene(p + h * n);
        occ += (h - d) * sca;
        sca *= 0.92;
    }
    return clamp(1.0 - 2.5 * occ, 0.0, 1.0);
}

// ============================================================================
// [11] 2D PLANET RENDERER
// ----------------------------------------------------------------------------
// Each planet is a 2D billboard with fake sphere lighting. The textured surface
// mixes voronoi-driven "ice crack" patterns with fbm continents. A halo bleeds
// out around the disc for atmosphere.
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
    float blurAmount   // 0 = sharp, 1 = very soft (for distant/blurred planets)
) {
    PlanetResult res;
    res.color = vec3(0.0);
    res.alpha = 0.0;
    res.halo = vec3(0.0);

    vec2 p = uv - center;
    float d = length(p);

    // Soft disc edge — wider falloff when blur is higher
    float edgeWidth = 0.003 + blurAmount * 0.04;
    float mask = smoothstep(radius + edgeWidth, radius - edgeWidth, d);

    // Halo — atmospheric scatter outside the disc
    float haloFalloff = 8.0 - blurAmount * 4.0;
    float halo = exp(-(d - radius) * haloFalloff);
    halo = max(halo, 0.0);
    float haloLit = smoothstep(-radius * 0.5, radius * 1.5, dot(p, lightDir));
    res.halo = iceColor * halo * haloLit * (0.5 + 0.3 * (1.0 - blurAmount));

    if (mask <= 0.001) return res;

    // Fake sphere normal
    float z = sqrt(max(0.0, radius * radius - d * d));
    vec3 normal = normalize(vec3(p.x, p.y, z));
    vec3 L = normalize(vec3(lightDir.x, lightDir.y, 0.6));

    // Surface UV — wrap around sphere
    vec2 sp = normal.xy * 1.8 + vec2(surfaceSeed * 11.3, surfaceSeed * 7.7);
    sp += vec2(u_time * 0.005, 0.0); // very slow rotation

    // Continents from domain-warped fbm
    float continents = domainWarp2D(sp * 2.5, u_time * 0.01);

    // Ice cracks from voronoi F1
    vec3 v = voronoi2D(sp * 4.0);
    float cracks = smoothstep(0.0, 0.15, v.x);

    // Polar caps — brighter near top/bottom
    float pole = pow(abs(normal.y), 1.5);

    // Combine: base color + ice patches in cracks + continents
    vec3 surface = mix(baseColor, iceColor, smoothstep(0.4, 0.7, continents));
    surface = mix(iceColor * 1.1, surface, cracks);  // crack lines are bright ice
    surface = mix(surface, vec3(0.95, 0.97, 1.0), pole * 0.6);

    // Lighting
    float diff = max(dot(normal, L), 0.0);
    float wrapDiff = max(0.0, (dot(normal, L) + 0.3) / 1.3);
    float NdotV = max(normal.z, 0.0);

    // Rim light (atmospheric)
    float rim = pow(1.0 - NdotV, 3.0);

    // Specular
    vec3 H = normalize(L + vec3(0.0, 0.0, 1.0));
    float spec = pow(max(dot(normal, H), 0.0), 32.0) * (1.0 - cracks) * 0.4;

    vec3 final = surface * (wrapDiff * 0.9 + 0.15);
    final += iceColor * rim * diff * 0.8;
    final += vec3(1.0) * spec;

    // Apply blur by mixing toward fog tone (cheap radial blur approximation)
    final = mix(final, FOG_COLOR * 1.1, blurAmount * 0.5);

    res.color = final;
    res.alpha = mask;
    return res;
}

// ============================================================================
// [12] STARFIELD
// ----------------------------------------------------------------------------
// Multi-layer parallax stars. Bright stars get cross-shaped glints. Twinkling
// driven by per-star hash to avoid synchronized blinking.
// ============================================================================
vec3 renderStars(vec2 uv, vec2 pan) {
    vec3 col = vec3(0.0);

    for (float i = 1.0; i <= 4.0; i += 1.0) {
        float scale = 25.0 + i * 18.0;
        vec2 sp = uv * scale + pan * (0.3 + i * 0.15);
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i * 12.34);

        if (h > 0.93) {
            float brightness = (h - 0.93) * 14.0;
            float r = length(fp);

            // Twinkle — each star has its own phase
            float phase = h * 137.5 + u_time * (1.5 + h * 3.0);
            float twinkle = 0.6 + 0.4 * sin(phase);

            // Core glow
            float core = exp(-r * (40.0 - brightness * 8.0));

            // Bright stars get cross glints
            float glint = 0.0;
            if (brightness > 0.6) {
                float gx = exp(-abs(fp.y) * 100.0) * exp(-abs(fp.x) * 8.0);
                float gy = exp(-abs(fp.x) * 100.0) * exp(-abs(fp.y) * 8.0);
                glint = (gx + gy) * 0.4 * (brightness - 0.6);
            }

            // Color: most cool, occasional warm
            vec3 starCol = mix(STAR_COOL, STAR_WARM, fract(h * 33.3));
            col += starCol * (core + glint) * twinkle * brightness;
        }
    }
    return col;
}

// ============================================================================
// [13] NEBULA
// ----------------------------------------------------------------------------
// Volumetric-feeling nebula via 3D domain-warped fbm. Z slice drifts with time
// so the cloud shapes evolve continuously.
// ============================================================================
vec3 renderNebula(vec2 uv, vec2 pan, vec2 flow) {
    vec3 warpPos = vec3(uv * 1.2 + pan * 0.4 - flow * 0.6, u_time * 0.015);
    float density = domainWarp3D(warpPos, u_time * 0.008);
    density = smoothstep(0.3, 0.85, density);

    // Secondary fine detail
    float detail = fbm3D(warpPos * 3.5 + vec3(2.3, 1.7, 0.0));
    density *= 0.7 + 0.3 * detail;

    // Color variation across the cloud
    float colorMix = fbm3D(warpPos * 2.0 + vec3(7.1, 3.4, 0.0));
    vec3 col = mix(NEBULA_A, NEBULA_B, colorMix);

    // Bright cores where density is highest
    float cores = smoothstep(0.7, 0.95, density);
    col = mix(col, vec3(0.85, 0.92, 1.0), cores * 0.6);

    return col * density;
}

// ============================================================================
// [14] CLOUD BANDS
// ----------------------------------------------------------------------------
// Three vertically-stacked cloud strata at different depths. They're what give
// the scene its dense, weatherly atmosphere — and what hide the planets'
// silhouettes enough to keep things calm.
// ============================================================================
float cloudBand(vec2 uv, float yCenter, float yWidth, float scale, float scrollSpeed, float seed) {
    // Distance from band center along Y
    float bandMask = exp(-pow((uv.y - yCenter) / yWidth, 2.0));

    vec2 cp = uv * scale + vec2(u_time * scrollSpeed + seed, seed * 0.7);
    float density = domainWarp2D(cp, u_time * 0.02);
    density = smoothstep(0.35, 0.75, density);

    // Fine detail
    density *= 0.7 + 0.3 * fbm2D(cp * 3.0);

    return density * bandMask;
}

// ============================================================================
// [15] SNOW PARTICLES
// ----------------------------------------------------------------------------
// Six layers. Far layers: small/slow/dim. Near layers: chunky/fast/streaked
// by pan_velocity for motion blur.
// ============================================================================
float snowLayer(vec2 uv, float scale, float speed, float seed, vec2 drift, vec2 motionBlur, float threshold) {
    vec2 sp = uv * scale;
    sp.y += u_time * speed;
    sp += drift;

    // Per-column horizontal wobble — wind variability
    sp.x += sin(sp.y * 1.7 + seed * 6.0 + u_time * 0.5) * 0.3;
    sp.x += sin(sp.y * 0.4 + seed * 2.0) * 0.6;

    vec2 id = floor(sp);
    vec2 fp = fract(sp) - 0.5;

    float h = hash12(id + seed);
    if (h < threshold) return 0.0;

    // Motion blur — stretch local space along velocity
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
    // Far
    total += snowLayer(uv, 22.0, 0.08, 3.1,  pan * 0.3 + flow * 0.4, vel * 0.2, 0.85) * 0.4;
    total += snowLayer(uv, 17.0, 0.13, 5.7,  pan * 0.4 + flow * 0.6, vel * 0.3, 0.84) * 0.55;
    // Mid
    total += snowLayer(uv, 12.0, 0.22, 7.7,  pan * 0.6 + flow * 0.9, vel * 0.5, 0.82) * 0.7;
    total += snowLayer(uv,  9.0, 0.30, 11.3, pan * 0.8 + flow * 1.2, vel * 0.7, 0.80) * 0.85;
    // Near
    total += snowLayer(uv,  6.0, 0.42, 13.9, pan * 1.1 + flow * 1.6, vel * 0.9, 0.78) * 0.95;
    total += snowLayer(uv,  3.5, 0.60, 21.3, pan * 1.5 + flow * 2.2, vel * 1.2, 0.75) * 0.7;
    return vec3(0.94, 0.97, 1.0) * total;
}

// ============================================================================
// [16] POST-PROCESS HELPERS
// ============================================================================
vec3 reinhardTonemap(vec3 c) {
    return c / (1.0 + c);
}

vec3 filmicTonemap(vec3 x) {
    // Mild filmic — less crushing than ACES, preserves the soft mid-range
    // we want for a non-obstructive background.
    float a = 2.0;
    float b = 0.10;
    float c = 1.9;
    float d = 0.45;
    float e = 0.10;
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

// Heavy fog falloff curve — used both for the foreground fog veil and
// for the central calm zone. Quadratic-ish, weighted toward distance.
float fogFactor(float t, float density) {
    return 1.0 - exp(-t * density);
}

// ============================================================================
// [17] MAIN RENDER PIPELINE
// ============================================================================
void main() {
    // -------- Screen space setup --------
    vec2 fragUv = gl_FragCoord.xy / u_resolution.xy;
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution.xy) / u_resolution.y;

    // Zoom — gentle scale only. No spiral; spiraling under window movement is
    // disorienting and was the worst part of the original.
    float z = clamp(u_zoom, 0.4, 3.0);
    uv /= z;

    // Normalize the large pan/flow values into small parallax offsets.
    // These multipliers assume u_pan is in pixel-ish accumulated units;
    // tune in Rust if they're already normalized.
    vec2 pan = vec2(u_pan.x, -u_pan.y) * 0.0008;
    vec2 flow = u_flow_offset * 0.0006;
    vec2 vel = pan_velocity * 0.0006;

    // Clamp velocity magnitude so very fast pans don't smear infinitely
    float vlen = length(vel);
    if (vlen > 0.18) vel *= 0.18 / vlen;

    // ========================================================================
    // PASS 1 — Deep sky gradient
    // ========================================================================
    float vGrad = clamp(fragUv.y, 0.0, 1.0);
    vec3 col = mix(HORIZON_COLOR, DEEP_SKY_COLOR, smoothstep(0.15, 0.95, vGrad));

    // Subtle horizontal gradient for asymmetry (light source bias)
    col += vec3(0.02, 0.03, 0.05) * (1.0 - fragUv.x);

    // ========================================================================
    // PASS 2 — Nebula (behind stars)
    // ========================================================================
    vec3 nebula = renderNebula(uv, pan, flow);
    col += nebula * 0.6;

    // ========================================================================
    // PASS 3 — Starfield
    // ========================================================================
    vec3 stars = renderStars(uv, pan);
    col += stars;

    // ========================================================================
    // PASS 4 — Planets (back to front)
    // ========================================================================
    vec2 lightDir2D = normalize(vec2(0.7, 0.55));

    // Planet 1 — small distant upper right (the little blue moon in the corner)
    {
        PlanetResult pl = renderPlanet(
            uv,
            vec2(0.85, 0.45) - pan * 0.15,
            0.08,
            vec3(0.18, 0.28, 0.42),
            vec3(0.62, 0.78, 0.92),
            lightDir2D,
            3.7,
            0.15
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    // Planet 2 — small moon-companion upper-mid-left
    {
        PlanetResult pl = renderPlanet(
            uv,
            vec2(-0.25, 0.55) - pan * 0.25,
            0.07,
            vec3(0.22, 0.30, 0.40),
            vec3(0.70, 0.82, 0.95),
            lightDir2D,
            7.1,
            0.1
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    // Planet 3 — large blurred middle-distance left (the partially-cloud-covered ice giant)
    {
        PlanetResult pl = renderPlanet(
            uv,
            vec2(-0.65, 0.20) - pan * 0.35,
            0.28,
            vec3(0.14, 0.22, 0.36),
            vec3(0.66, 0.80, 0.92),
            lightDir2D,
            11.3,
            0.25
        );
        col += pl.halo * 0.7;
        col = mix(col, pl.color, pl.alpha * 0.92);
    }

    // Planet 4 — mid-center small ice planet (between the big two)
    {
        PlanetResult pl = renderPlanet(
            uv,
            vec2(0.05, -0.05) - pan * 0.5,
            0.13,
            vec3(0.18, 0.26, 0.38),
            vec3(0.70, 0.84, 0.96),
            lightDir2D,
            17.9,
            0.18
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    // Planet 5 — large foreground right (the dominant ice planet)
    {
        PlanetResult pl = renderPlanet(
            uv,
            vec2(0.75, -0.10) - pan * 0.6,
            0.42,
            vec3(0.20, 0.30, 0.44),
            vec3(0.78, 0.90, 1.0),
            lightDir2D,
            23.4,
            0.08
        );
        col += pl.halo;
        col = mix(col, pl.color, pl.alpha);
    }

    // ========================================================================
    // PASS 5 — Cloud bands (between planets and foreground)
    // ========================================================================
    // Mid-distance heavy cloud — gives the scene its weather
    float midClouds = cloudBand(
        uv + pan * 0.6 + flow * 1.2,
        -0.05, 0.55, 1.8, 0.04, 4.2
    );
    col = mix(col, vec3(0.86, 0.91, 0.97), midClouds * 0.55);

    // Lower cloud band — drifting fog at bottom
    float lowerClouds = cloudBand(
        uv + pan * 0.8 + flow * 1.6,
        -0.45, 0.4, 2.5, 0.06, 9.1
    );
    col = mix(col, vec3(0.90, 0.94, 0.98), lowerClouds * 0.65);

    // Upper thin haze — barely there, just to soften the sky
    float upperHaze = cloudBand(
        uv + pan * 0.4 + flow * 0.7,
        0.45, 0.35, 1.2, 0.025, 13.7
    );
    col = mix(col, vec3(0.75, 0.82, 0.92), upperHaze * 0.25);

    // ========================================================================
    // PASS 6 — Foreground raymarched ice crystals
    // ========================================================================
    // Ray setup — camera looks slightly down so foreground crystals sit at the bottom
    vec3 ro = vec3(pan.x * 0.4, -pan.y * 0.4 + 0.1, -4.0);
    vec3 rd = normalize(vec3(uv * 1.2, 1.0));
    // Very subtle camera pitch toward the bottom
    rd = rotX(-0.05) * rd;

    float t = 0.5;
    bool hit = false;
    for (int i = 0; i < MAX_STEPS; i++) {
        vec3 p = ro + rd * t;
        float d = mapScene(p);
        if (d < SURF_DIST) { hit = true; break; }
        t += d * 0.9;  // step factor < 1 for better quality on sharp edges
        if (t > MAX_DIST) break;
    }

    if (hit) {
        vec3 p = ro + rd * t;
        vec3 n = calcNormal(p);
        vec3 v = normalize(ro - p);

        Light light;
        light.direction = normalize(vec3(0.7, 0.6, -0.5));
        light.color = vec3(0.95, 0.97, 1.0);
        light.intensity = 2.2;

        Material ice;
        ice.albedo = ICE_ALBEDO;
        ice.roughness = 0.08;
        ice.metallic = 0.05;
        ice.transmission = 0.7;

        vec3 pbr = calculatePBR(n, v, light.direction, ice, light);
        float ao = calcAO(p, n);
        float sh = softShadow(p, light.direction, 0.04, 4.0, 12.0);

        // Subsurface scattering — light coming through the back
        float sss = pow(max(0.0, dot(v, -light.direction)), 4.0) * 0.5;
        vec3 sssCol = ICE_DEEP * sss * 1.2;

        // Internal refraction — sample the background behind, distorted by normal
        vec3 refRay = refract(rd, n, 1.0 / IOR_ICE);
        vec2 refUv = uv + refRay.xy * 0.3;
        // Sample fog density at the distorted coord for a refracted background
        float refFog = domainWarp2D(refUv * 2.5 + flow, u_time * 0.01);
        vec3 refColor = mix(ICE_DEEP, FOG_COLOR, smoothstep(0.3, 0.7, refFog));

        // Fresnel — strong rim glow that sells the icy look
        vec3 F0 = vec3(0.04);
        vec3 fresnel = fresnelSchlick(max(dot(n, v), 0.0), F0);
        float rim = pow(1.0 - max(dot(n, v), 0.0), 3.0);

        // Composite material
        vec3 crystalCol = pbr * sh * ao;
        crystalCol += sssCol * ao;
        crystalCol += refColor * 0.5 * (1.0 - rim);
        crystalCol += vec3(0.9, 0.95, 1.0) * rim * 0.8;
        crystalCol = mix(crystalCol, ICE_ALBEDO * 0.7, 0.2);  // cool base tint

        // Distance fog — far crystals fade into the haze
        float fogAmount = fogFactor(t, FOG_DENSITY);
        crystalCol = mix(crystalCol, FOG_COLOR, fogAmount);

        // Depth-of-field blur falloff — focal plane at t≈4, fade out beyond
        float dofFocal = 3.5;
        float dofRange = 6.0;
        float dofBlur = smoothstep(0.0, dofRange, abs(t - dofFocal));

        // Blend crystals over background. dofBlur pushes blurry crystals toward bg.
        col = mix(crystalCol, col, dofBlur * 0.7);
    }

    // ========================================================================
    // PASS 7 — Snowfall (over everything except final post)
    // ========================================================================
    col += renderSnow(uv, pan, flow, vel);

    // ========================================================================
    // PASS 8 — Volumetric fog veil
    // ----------------------------------------------------------------------------
    // This is the master non-obstructive trick. A thick warped fog layer mixed
    // OVER the entire scene at moderate alpha. Reduces contrast everywhere,
    // unifies the palette, and is what makes the planets/crystals feel hazy
    // and distant rather than crisp and demanding.
    // ========================================================================
    vec2 veilUv = uv * 0.8 + pan * 0.5 + flow * 1.0 + vec2(u_time * 0.01, u_time * 0.005);
    float veilDensity = domainWarp2D(veilUv, u_time * 0.015);
    veilDensity = smoothstep(0.2, 0.8, veilDensity);

    // The veil is stronger in the middle vertically (where action concentrates)
    // and weaker at the very top (preserves dark sky for stars).
    float veilVerticalMask = smoothstep(0.0, 0.35, fragUv.y) * smoothstep(1.0, 0.65, fragUv.y);
    veilVerticalMask = max(veilVerticalMask, 0.45);  // baseline veil everywhere

    float veilAmount = veilDensity * veilVerticalMask * 0.25;
    col = mix(col, FOG_COLOR, veilAmount);

    // ========================================================================
    // PASS 9 — Central calm zone
    // ----------------------------------------------------------------------------
    // Radial mask that pulls the central region toward a flat luminance, so
    // any UI rendered on top has uniform contrast underneath. The edges keep
    // their punch; the middle softens.
    // ========================================================================
    float distFromCenter = length(fragUv - 0.5);
    float calmMask = 1.0 - smoothstep(CALM_RADIUS - 0.15, CALM_RADIUS + 0.15, distFromCenter);
    vec3 calmTarget = FOG_COLOR * 0.9;
    col = mix(col, calmTarget, calmMask * CALM_STRENGTH * 0.35);

    // ========================================================================
    // PASS 10 — Final color grading & post
    // ========================================================================

    // Filmic tonemap
    col = filmicTonemap(col);

    // Slight desaturation — non-obstructive backgrounds shouldn't be vivid
    col = desaturate(col, 0.18);

    // Cool color grade — push midtones slightly cyan-blue
    vec3 coolTint = vec3(0.95, 0.99, 1.05);
    col *= coolTint;

    // Vignette — gentle, only at the very corners
    float vig = vignette(fragUv, 0.92, 0.6);
    col *= mix(0.7, 1.0, vig);

    // Subtle film grain — kills gradient banding
    float grain = (hash12(gl_FragCoord.xy + fract(u_time)) - 0.5) * 0.014;
    col += grain;

    // Lift blacks slightly — prevents the deep sky from being pure black
    col = max(col, vec3(0.04, 0.05, 0.07));

    // Final clamp
    col = clamp(col, 0.0, 1.0);

    gl_FragColor = vec4(col, 1.0) * alpha;
}
