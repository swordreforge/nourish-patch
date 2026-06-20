precision highp float;

// ============================================================================
// UNIFORMS
// ============================================================================
uniform float u_time;
uniform vec2  u_pan;          // accumulated pan (pixels-ish, large numbers)
uniform vec2  u_flow_offset;  // accumulated flow drift
uniform vec2  pan_velocity;   // instantaneous pan velocity
uniform float u_zoom;         // 1.0 = neutral
uniform vec2  u_resolution;
uniform float alpha;          // injected by smithay

// ============================================================================
// CONSTANTS
// ============================================================================
#define PI 3.14159265359

// ============================================================================
// HASH / NOISE (kept small â€” only what's actually used)
// ============================================================================
float hash12(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec2 hash22(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = hash12(i);
    float b = hash12(i + vec2(1.0, 0.0));
    float c = hash12(i + vec2(0.0, 1.0));
    float d = hash12(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
    for (int i = 0; i < 5; i++) {
        v += a * vnoise(p);
        p = m * p;
        a *= 0.5;
    }
    return v;
}

// Domain-warped fbm for soft clouds / fog
float warpedFbm(vec2 p) {
    vec2 q = vec2(fbm(p), fbm(p + vec2(5.2, 1.3)));
    return fbm(p + 2.0 * q);
}

mat2 rot2(float a) {
    float s = sin(a);
    float c = cos(a);
    return mat2(c, -s, s, c);
}

// ============================================================================
// ICE SHARD SDF (2D)
// ----------------------------------------------------------------------------
// Tall, narrow diamond â€” the kind of crystal shape that reads as "shard"
// even when small and blurred.
// ============================================================================
float sdShard(vec2 p, float w, float h) {
    p = abs(p);
    // Two slanted edges meeting at top, flat-ish bottom
    float d = max(p.x / w + p.y / h - 1.0, p.y / h - 0.85);
    return d;
}

// One layer of scattered ice shards, returned as a soft coverage mask + glint
// `scale` controls density (higher = more, smaller shards)
// `seed`  decorrelates layers
// `softness` controls how blurred each shard is (depth cue)
vec2 shardLayer(vec2 uv, float scale, float seed, float softness) {
    vec2 sp = uv * scale;
    vec2 id = floor(sp);
    vec2 fp = fract(sp) - 0.5;

    vec2 h2 = hash22(id + seed);
    float h  = hash12(id + seed * 1.37);

    // Cull most cells so shards stay sparse
    if (h < 0.55) {
        return vec2(0.0);
    }

    // Per-shard jitter inside its cell
    vec2 offset = (h2 - 0.5) * 0.6;
    vec2 q = fp - offset;

    // Random rotation & aspect
    float ang = (h2.x - 0.5) * PI * 0.9;
    q = rot2(ang) * q;

    float w = 0.05 + h2.y * 0.07;
    float hgt = 0.18 + h * 0.22;

    float d = sdShard(q, w, hgt);

    // Soft edge â€” wider falloff = more blur = further away
    float mask = smoothstep(softness, -softness * 0.3, d);

    // A subtle inner glint along one edge of the shard
    float glint = smoothstep(softness * 1.5, 0.0, abs(d + 0.02)) * (0.4 + 0.6 * h2.x);

    return vec2(mask, glint * mask);
}

// ============================================================================
// SNOWFLAKES
// ----------------------------------------------------------------------------
// One layer of drifting snow. Returns coverage [0,1].
// Motion blur stretches flakes in the direction of pan_velocity.
// ============================================================================
float snowLayer(vec2 uv, float scale, float speed, float seed, vec2 drift, vec2 motionBlur) {
    // Move the whole layer downward over time, plus drift from flow/wind
    vec2 sp = uv * scale;
    sp.y += u_time * speed;
    sp += drift;

    // Per-column horizontal wobble so flakes don't fall in straight lines
    sp.x += sin(sp.y * 1.7 + seed * 6.0 + u_time * 0.6) * 0.35;

    vec2 id = floor(sp);
    vec2 fp = fract(sp) - 0.5;

    float h = hash12(id + seed);
    if (h < 0.82) return 0.0;

    // Stretch the local space along velocity to fake motion blur
    vec2 mb = motionBlur / max(scale, 1.0);
    float mbLen = length(mb);
    if (mbLen > 0.001) {
        vec2 dir = mb / mbLen;
        // Project onto/perpendicular to dir, stretch the parallel axis
        float par = dot(fp, dir);
        float per = dot(fp, vec2(-dir.y, dir.x));
        par /= (1.0 + mbLen * 8.0);
        fp = dir * par + vec2(-dir.y, dir.x) * per;
    }

    float r = length(fp);
    float size = 0.05 + (h - 0.82) * 0.8;
    float core = smoothstep(size, 0.0, r);
    float halo = smoothstep(size * 3.0, size, r) * 0.25;
    return core + halo;
}

// ============================================================================
// MAIN
// ============================================================================
void main() {
    vec2 fragUv = gl_FragCoord.xy / u_resolution.xy;

    // Aspect-corrected centered UV. Y up.
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution.xy) / u_resolution.y;

    // ----- Zoom: gentle scale instead of spiral. Clamp so extremes don't break it. -----
    float z = clamp(u_zoom, 0.5, 2.5);
    uv /= z;

    // ----- Normalize pan to small parallax offsets (raw u_pan can be huge) -----
    // Different layers will multiply this by different parallax factors.
    vec2 pan = vec2(u_pan.x, -u_pan.y) * 0.0008;
    vec2 flow = u_flow_offset * 0.0005;

    // Smooth motion-blur vector from instantaneous velocity
    vec2 vel = pan_velocity * 0.0005;
    // Cap so very fast pans don't smear into infinity
    float vlen = length(vel);
    if (vlen > 0.15) vel *= 0.15 / vlen;

    // ============================================================
    // LAYER 1 â€” Sky gradient (top dark blue â†’ bottom pale)
    // Top-down feel: cooler/darker high, warmer/brighter low where snow accumulates.
    // ============================================================
    float vGrad = clamp(fragUv.y, 0.0, 1.0);
    vec3 skyTop = vec3(0.18, 0.24, 0.34);   // deep cold sky
    vec3 skyMid = vec3(0.55, 0.66, 0.78);   // hazy band
    vec3 skyLow = vec3(0.82, 0.88, 0.94);   // bright snow-glow horizon
    vec3 col = mix(skyLow, skyMid, smoothstep(0.0, 0.55, vGrad));
    col = mix(col, skyTop, smoothstep(0.45, 1.0, vGrad));

    // ============================================================
    // LAYER 2 â€” Soft volumetric clouds / fog
    // Drifts slowly with flow + pan. Concentrated in mid-vertical band so the
    // very top stays clean (good for UI) and the bottom gets soft luminance.
    // ============================================================
    vec2 cloudUv = uv * 1.3 + pan * 0.6 + flow * 2.0 + vec2(u_time * 0.015, 0.0);
    float clouds = warpedFbm(cloudUv);
    clouds = smoothstep(0.35, 0.85, clouds);

    // Vertical mask so clouds don't smear over the top UI area
    float cloudBand = smoothstep(0.05, 0.5, fragUv.y) * smoothstep(1.0, 0.5, fragUv.y);
    // Always a soft floor of fog at the bottom
    cloudBand = max(cloudBand, smoothstep(0.25, 0.0, fragUv.y) * 0.7);

    vec3 cloudColor = vec3(0.88, 0.93, 0.98);
    col = mix(col, cloudColor, clouds * cloudBand * 0.55);

    // ============================================================
    // LAYER 3 â€” Distant ice shards (very blurred, low contrast)
    // Lives far away â†’ slowest parallax, big softness.
    // ============================================================
    {
        vec2 luv = uv + pan * 0.3;
        vec2 s = shardLayer(luv, 3.5, 11.1, 0.06);
        // Tint shards: cool blue-white, low alpha because they're distant
        vec3 sc = mix(vec3(0.65, 0.74, 0.85), vec3(0.95, 0.98, 1.0), s.y);
        col = mix(col, sc, s.x * 0.22);
    }

    // ============================================================
    // LAYER 4 â€” Mid ice shards
    // ============================================================
    {
        vec2 luv = uv + pan * 0.6 + vec2(0.4, -0.2);
        vec2 s = shardLayer(luv, 5.5, 27.3, 0.035);
        vec3 sc = mix(vec3(0.72, 0.82, 0.92), vec3(1.0), s.y);
        col = mix(col, sc, s.x * 0.32);
    }

    // ============================================================
    // LAYER 5 â€” Near ice shards (still soft, never sharp â€” non-obstructive)
    // Lives in lower half mostly, so the top stays clean.
    // ============================================================
    {
        vec2 luv = uv + pan * 1.0 + vec2(-0.3, 0.5);
        vec2 s = shardLayer(luv, 7.5, 53.7, 0.025);
        // Fade out near the top so UI region stays clean
        float topFade = smoothstep(0.85, 0.4, fragUv.y);
        vec3 sc = mix(vec3(0.78, 0.86, 0.95), vec3(1.0), s.y);
        col = mix(col, sc, s.x * 0.38 * topFade);
    }

    // ============================================================
    // LAYER 6 â€” Snowfall, multiple depths
    // Far layers: small, slow, no motion blur.
    // Near layers: bigger, faster, stretched by pan_velocity.
    // ============================================================
    float snowAccum = 0.0;
    // Far
    snowAccum += snowLayer(uv, 18.0, 0.10, 3.1, pan * 0.4 + flow * 0.5, vel * 0.3) * 0.55;
    // Mid
    snowAccum += snowLayer(uv, 12.0, 0.18, 7.7, pan * 0.7 + flow * 1.0, vel * 0.7) * 0.75;
    // Near
    snowAccum += snowLayer(uv,  7.0, 0.32, 13.9, pan * 1.1 + flow * 1.5, vel * 1.0) * 1.0;
    // Very near (chunky drift, sparse)
    snowAccum += snowLayer(uv,  4.0, 0.55, 21.3, pan * 1.6 + flow * 2.2, vel * 1.4) * 0.85;

    col += vec3(0.95, 0.97, 1.0) * snowAccum * 0.9;

    // ============================================================
    // FINAL â€” keep it low-contrast and non-obstructive
    // ============================================================
    // Very gentle tonemap (no ACES â€” we want flat, soft)
    col = col / (1.0 + col * 0.25);

    // Subtle vignette, low strength so top corners don't go dark
    float vig = 1.0 - length((fragUv - 0.5) * vec2(0.9, 0.7)) * 0.35;
    col *= vig;

    // Light grain to avoid banding on the gradient
    float grain = (hash12(gl_FragCoord.xy + u_time) - 0.5) * 0.012;
    col += grain;

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0) * alpha;
}