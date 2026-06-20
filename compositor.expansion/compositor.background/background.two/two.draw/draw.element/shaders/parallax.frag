precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2 pan_velocity;
uniform vec2 u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// --- Hash & noise ---------------------------------------------------------
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        v += a * noise(p);
        p *= 2.0;
        a *= 0.5;
    }
    return v;
}

// --- Creature SDF (jellyfish-ish blob) ------------------------------------
float sd_creature(vec2 p, float s) {
    // body + wobbly tendrils
    float body = length(p) - s;
    float tendrils = 0.0;
    for (int i = 0; i < 3; i++) {
        float fi = float(i);
        vec2 q = p - vec2(0.0, -s - 0.05 * fi);
        q.x += sin(u_time * 1.5 + fi * 2.0 + p.y * 8.0) * 0.02;
        tendrils = max(tendrils, smoothstep(0.02, 0.0, abs(q.x)) *
                                 smoothstep(s + 0.15, s, abs(q.y + s)));
    }
    return min(body, -tendrils * 0.05);
}

// --- Bubble particles -----------------------------------------------------

float bubbles(vec2 uv, float depth, vec2 flow_off) {
    vec2 flow = vec2(-flow_off.x / depth, -u_time * 0.05 * depth);
    vec2 p = uv * (8.0 / depth) + flow;
    // Bubbles rise (negative y) and drift with negative pan velocity
    // vec2 flow = vec2(-vel.x * 0.0005, -u_time * 0.05 * depth);
    // vec2 p = uv * (8.0 / depth) + flow;

    vec2 i = floor(p);
    vec2 f = fract(p) - 0.5;

    float h = hash(i);
    if (h < 0.6) return 0.0;            // sparse

    // jitter bubble inside its cell
    vec2 off = vec2(hash(i + 1.3) - 0.5, hash(i + 2.7) - 0.5) * 0.6;
    float r  = 0.04 + 0.08 * hash(i + 5.1);
    float d  = length(f - off) - r;
    return smoothstep(0.01, -0.01, d);
}

void main() {
    // Aspect-correct UVs centered on screen
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;
    uv /= u_zoom;

    // --- Background gradient: deep water ---
    float depthGrad = (gl_FragCoord.y / u_resolution.y);          // 0 bottom -> 1 top
    vec3 deep    = vec3(0.01, 0.03, 0.08);
    vec3 shallow = vec3(0.05, 0.20, 0.35);
    vec3 col = mix(deep, shallow, depthGrad);

    // God-rays from the top
    float rays = 0.0;
    for (int i = 0; i < 4; i++) {
        float fi = float(i);
        float x = uv.x * (1.5 + fi * 0.3) + sin(u_time * 0.2 + fi) * 0.4;
        rays += smoothstep(0.02, 0.0, abs(fract(x) - 0.5)) * (1.0 - depthGrad);
    }
    col += vec3(0.15, 0.25, 0.35) * rays * 0.08;

    // --- Parallax creature layers (back to front) ---
    for (int i = 1; i <= 4; i++) {
        float fi = float(i);
        float depth = fi * 0.25;                     // closer = larger depth
        vec2 layer_uv = uv + u_pan * depth * 0.0008;
        layer_uv += vec2(sin(layer_uv.y * 3.0 + u_time * 0.5),
                         cos(layer_uv.x * 3.0 + u_time * 0.7)) * 0.015;

        // tile creatures across the layer
        vec2 cell = vec2(1.2, 1.5);
        vec2 id   = floor(layer_uv / cell);
        vec2 lp   = mod(layer_uv, cell) - cell * 0.5;
        lp += vec2(hash(id) - 0.5, hash(id + 3.7) - 0.5) * 0.6;

        float size = 0.04 + 0.04 * depth;
        float dist = sd_creature(lp, size);
        float mask = smoothstep(0.01, -0.005, dist);

        vec3  ctint = mix(vec3(0.20, 0.35, 0.55),
                          vec3(0.40, 0.60, 0.80), depth);
        col = mix(col, ctint, mask * 0.55 * depth);
    }

    // --- Caustics (top-down light shimmer) ---
    vec2 cUv = uv * 3.0 + u_pan * 0.0005;
    float c  = fbm(cUv + u_time * 0.15);
    c        = pow(c, 3.0);
    col     += vec3(0.20, 0.35, 0.45) * c * (1.0 - depthGrad) * 0.4;

    // --- Bubble particles, 3 parallax layers, react to pan velocity ---
    for (int i = 1; i <= 3; i++) {
        float fi = float(i);
        float depth = fi * 0.4;
        float b = bubbles(uv, depth, u_flow_offset);
        col += vec3(0.7, 0.85, 1.0) * b * 0.35 / depth;
    }

    // Subtle vignette
    float vig = smoothstep(1.2, 0.3, length(uv));
    col *= mix(0.6, 1.0, vig);

    gl_FragColor = vec4(col, 1.0) * alpha;
}