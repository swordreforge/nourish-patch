precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

float hash(float x){ return fract(sin(x*127.1)*43758.5453); }
float hash2(vec2 p){ return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
float vnoise(float x){
    float i=floor(x), f=fract(x);
    f=f*f*(3.0-2.0*f);
    return mix(hash(i), hash(i+1.0), f);
}

// ---- LOW-POLY CLOUDS: stacks of hard-edged boxes, solid white ----
// A cloud is a horizontal arrangement of rectangular "blocks" of varying height.
// We render with no gradients — fully solid pixels — and quantize the silhouette.
float box_in(vec2 p, vec2 size) {
    vec2 d = abs(p) - size;
    return (max(d.x, d.y) < 0.0) ? 1.0 : 0.0;
}

float cloud_block_stack(vec2 p, float seed) {
    // Cloud occupies roughly p.x in [-0.5, 0.5], p.y in [0, 0.15]
    // Composed of 5-7 boxes of varying widths/heights, slightly overlapping
    float c = 0.0;

    // Base row (widest, lowest) — the foundation
    c = max(c, box_in(p - vec2(0.0,  0.025), vec2(0.45, 0.025)));

    // Mid row — slightly narrower
    c = max(c, box_in(p - vec2(-0.10, 0.060), vec2(0.20, 0.020)));
    c = max(c, box_in(p - vec2( 0.12, 0.060), vec2(0.18, 0.018)));

    // Top row — narrower still, low peaks (NOT spiky)
    float w1 = 0.10 + 0.04 * hash(seed + 1.0);
    float w2 = 0.08 + 0.04 * hash(seed + 2.0);
    c = max(c, box_in(p - vec2(-0.05, 0.085), vec2(w1, 0.015)));
    c = max(c, box_in(p - vec2( 0.10, 0.085), vec2(w2, 0.012)));

    return c;
}

// Multi-cloud field with parallax depth
float cloud_field(vec2 uv, float depth, vec2 flow_off, float yBand) {
    // Always-moving horizontal drift
    float wind = u_time * 0.08 / depth + (-flow_off.x * 0.0008 / depth);
    vec2 cuv = uv * (1.0 / depth) + vec2(wind, 0.0);

    // Sparse spacing (clouds are not everywhere)
    vec2 cell = vec2(2.2, 0.5);
    vec2 id = floor(cuv / cell);
    vec2 lp = mod(cuv, cell) - cell*0.5;

    float h = hash2(id);
    if (h < 0.55) return 0.0;  // ~45% of cells have clouds — sparse

    // Vertical jitter within the band
    float yJit = (hash2(id + 5.7) - 0.5) * 0.08;
    vec2 cp = lp - vec2(0.0, yBand + yJit);

    return cloud_block_stack(cp, h * 31.0);
}

// Mountain top sample (smooth triangle-wave silhouette)
struct MtnSample { float top; float h01; };
MtnSample mountain_top(float x, float height, float base_y, float seed) {
    float i = floor(x);
    float f = fract(x);
    float peakA = hash(i + seed);
    float peakB = hash(i + 1.0 + seed);
    float jitter = 0.3 + 0.4 * hash(i + 7.0 + seed);
    float tri = f < jitter
        ? mix(peakA, 1.0, f / jitter)
        : mix(1.0, peakB, (f - jitter) / (1.0 - jitter));
    tri *= 0.55 + 0.45 * hash(i + 13.0 + seed);

    MtnSample r;
    r.top = base_y + tri * height;
    r.h01 = tri;
    return r;
}

void main() {
    // --- TWO COORDINATE SYSTEMS ---
    // screen_uv: raw screen UV, NOT affected by zoom. Used for sky/sun/clouds — these
    //   are "sky dome" features that shouldn't scale.
    // world_uv: zoomed UV, used for mountains' X (so panning still parallaxes correctly).
    //   But mountain Y is BOUNDED to screen_uv so we never see below them.
    vec2 screen_uv = (gl_FragCoord.xy - 0.5*u_resolution) / u_resolution.y;
    vec2 world_uv  = screen_uv / u_zoom;

    // The bottom of the screen in screen_uv terms
    float screen_bottom = -0.5;  // since y is in [-0.5, 0.5] for the visible viewport

    // -------- SKY (screen-space, unaffected by zoom) --------
    float skyT = clamp(screen_uv.y*1.0 + 0.5, 0.0, 1.0);
    vec3 horizon = vec3(0.95, 0.50, 0.42);
    vec3 mid     = vec3(0.55, 0.30, 0.50);
    vec3 zenith  = vec3(0.10, 0.08, 0.28);
    vec3 col = mix(horizon, mid, smoothstep(0.0, 0.5, skyT));
    col      = mix(col,     zenith, smoothstep(0.4, 1.0, skyT));

    // Sun (screen-space — fixed sky position)
    vec2 sunPos = vec2(0.15, 0.02);
    float sd = length(screen_uv - sunPos);
    col = mix(col, vec3(1.0, 0.92, 0.75), smoothstep(0.11, 0.0, sd) * 0.95);
    col += vec3(1.0, 0.65, 0.45) * smoothstep(0.55, 0.0, sd) * 0.18;

    // Stars (screen-space)
    vec2 sp = screen_uv*40.0;
    vec2 sid = floor(sp);
    if (hash2(sid) > 0.985 && screen_uv.y > 0.05) {
        col += vec3(0.9, 0.85, 1.0) * 0.5;
    }

    // -------- CLOUDS (screen-space + horizontal pan only) --------
    // Pan affects clouds horizontally but they don't scale with zoom
    vec2 cloud_uv = screen_uv + vec2(u_pan.x * 0.0006, 0.0);
    float cFar  = cloud_field(cloud_uv,                    1.6, u_flow_offset, 0.30);
    float cMid  = cloud_field(cloud_uv + vec2(7.0, 0.0),   1.1, u_flow_offset, 0.22);
    float cNear = cloud_field(cloud_uv + vec2(15.0, 0.0),  0.8, u_flow_offset, 0.15);

    // Solid white with slight warm tint near sun
    vec3 cloudColFar  = vec3(0.95, 0.90, 0.92);
    vec3 cloudColMid  = vec3(0.98, 0.85, 0.82);
    vec3 cloudColNear = vec3(1.00, 0.78, 0.72);
    col = mix(col, cloudColFar,  cFar);
    col = mix(col, cloudColMid,  cMid);
    col = mix(col, cloudColNear, cNear);

    // -------- MOUNTAINS --------
    // Each layer's silhouette is in world space (so it pans/zooms), BUT the layer
    // FILLS from its silhouette down to the screen bottom — so zooming never reveals
    // empty area below.
    vec3 baseCols[5];
    baseCols[0] = vec3(0.50, 0.42, 0.58);
    baseCols[1] = vec3(0.40, 0.32, 0.50);
    baseCols[2] = vec3(0.30, 0.24, 0.42);
    baseCols[3] = vec3(0.22, 0.18, 0.32);
    baseCols[4] = vec3(0.14, 0.12, 0.24);

    float depths[5];  depths[0]=2.0; depths[1]=1.4; depths[2]=1.0; depths[3]=0.7; depths[4]=0.45;
    float heights[5]; heights[0]=0.08; heights[1]=0.10; heights[2]=0.12; heights[3]=0.14; heights[4]=0.18;
    // Bases now in WORLD space — they'll appear at the same world position regardless of zoom
    float bases[5];   bases[0]=0.05; bases[1]=-0.02; bases[2]=-0.10; bases[3]=-0.18; bases[4]=-0.28;
    float seeds[5];   seeds[0]=0.0; seeds[1]=11.0; seeds[2]=23.0; seeds[3]=37.0; seeds[4]=53.0;

    for (int i=0;i<5;i++){
        // World-x with parallax for the silhouette
        float x = world_uv.x * (2.0 / depths[i]) + u_pan.x * 0.0008 * depths[i] + seeds[i];
        MtnSample s = mountain_top(x, heights[i], bases[i], seeds[i]);

        // Paint everywhere FROM silhouette top DOWN TO screen bottom.
        // No floor — the mountain "body" fills to infinity below (offscreen).
        // But because we anchor to screen_bottom in SCREEN space, zooming out
        // just means the mountain body extends further visually — never reveals sky.
        float mask = smoothstep(0.004, -0.004, world_uv.y - s.top);
        if (mask <= 0.0) continue;

        vec3 mcol = baseCols[i];

        // Subtle vertical shading (top brighter, fades toward base)
        // Use distance below silhouette in screen units so it doesn't stretch with zoom
        float depthBelow = (s.top - world_uv.y);
        mcol *= mix(1.0, 0.65, smoothstep(0.0, 0.4, depthBelow));

        // Snow only at the upper portion of each peak
        float snow = smoothstep(0.65, 0.95, s.h01)
                   * smoothstep(0.04, 0.0, s.top - world_uv.y);
        mcol = mix(mcol, vec3(0.95, 0.90, 0.95), snow * 0.85);

        col = mix(col, mcol, mask);
    }

    // -------- HAZE (in front of mountains, near horizon) --------
    vec2 hazeFlow = vec2(-u_flow_offset.x*0.0004 + u_time*0.04, 0.0);
    float h = vnoise(screen_uv.x*3.0 + hazeFlow.x) * 0.5
            + vnoise(screen_uv.x*7.0 + hazeFlow.x*1.7) * 0.5;
    h *= smoothstep(0.05, -0.10, screen_uv.y) * smoothstep(-0.30, -0.10, screen_uv.y);
    col = mix(col, vec3(0.65, 0.50, 0.60), h * 0.15);

    gl_FragColor = vec4(col, 1.0) * alpha;
}