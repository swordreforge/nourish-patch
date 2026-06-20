// Parallax space background — native Vulkan fragment shader (HLSL → SPIR-V via
// glslang). Port of the GLES `spacev3.frag`; runs in a real VkPipeline so the
// Vulkan shader path is exercised. Uniforms arrive as push constants (packed
// into float4s for clean 16-byte alignment).
struct PushData {
    float4 res_zoom_time; // xy = resolution, z = zoom, w = time
    float4 pan_flow;      // xy = pan,        zw = flow_offset
    float4 lock_alpha;    // x  = lock_amount, y = alpha
};
// Must be `[[vk::push_constant]] ConstantBuffer<T>` — a plain attributed struct
// var makes glslang emit a descriptor cbuffer (DescriptorSet/Binding) instead of
// a real PushConstant, which our descriptor-less pipeline never binds (→ garbage
// uniforms → NaN → blank/noise output).
[[vk::push_constant]] ConstantBuffer<PushData> pc;

float hash(float2 p) {
    return frac(sin(dot(p, float2(127.1, 311.7))) * 43758.5453);
}
float noise(float2 p) {
    float2 i = floor(p), f = frac(p);
    f = f * f * (3.0 - 2.0 * f);
    return lerp(lerp(hash(i), hash(i + float2(1, 0)), f.x),
                lerp(hash(i + float2(0, 1)), hash(i + float2(1, 1)), f.x), f.y);
}
float fbm(float2 p) {
    float v = 0.0, a = 0.5;
    [unroll] for (int i = 0; i < 5; i++) { v += a * noise(p); p *= 2.0; a *= 0.5; }
    return v;
}
float sdCircle(float2 p, float r) { return length(p) - r; }

float3 draw_planet(float3 col, float2 uv, float2 center, float radius,
                   float3 lightSide, float3 darkSide, float2 lightDir, float bandFreq) {
    float2 pp = uv - center;
    float d = sdCircle(pp, radius);
    float mask = smoothstep(0.004, -0.004, d);
    if (mask <= 0.0) return col;
    float lit = smoothstep(-radius * 0.6, radius * 0.6, dot(pp, lightDir));
    float3 base = lerp(darkSide, lightSide, lit);
    if (bandFreq > 0.0) {
        float band = sin(pp.y * bandFreq + center.x * 3.0) * 0.5 + 0.5;
        float bandNoise = fbm(pp * 15.0) * 0.15;
        base = lerp(base, base * 0.75, smoothstep(0.2, 0.8, band + bandNoise));
    }
    float rim = smoothstep(radius * 0.5, radius, length(pp));
    float rimLit = smoothstep(-radius * 0.2, radius, dot(pp, lightDir));
    float3 atmosphere = lightSide * rim * rimLit * 0.5;
    return lerp(col, base + atmosphere, mask);
}

float galaxy(float2 uv, float2 c, float rot, float2 scale) {
    float2 p = uv - c;
    float s = sin(rot), co = cos(rot);
    p = float2(co * p.x - s * p.y, s * p.x + co * p.y);
    p /= scale;
    float r2 = dot(p, p);
    return exp(-r2 * 6.0) * 0.6 + exp(-r2 * 45.0) * 0.4; // halo + core
}

float4 main(float4 fragCoord : SV_Position) : SV_Target {
    float2 u_resolution = pc.res_zoom_time.xy;
    float u_zoom = pc.res_zoom_time.z;
    float u_time = pc.res_zoom_time.w;
    float2 u_pan = pc.pan_flow.xy;
    float2 u_flow_offset = pc.pan_flow.zw;
    float u_lock_amount = pc.lock_alpha.x;
    float alpha = pc.lock_alpha.y;

    float2 uv = (fragCoord.xy - 0.5 * u_resolution) / u_resolution.y;
    uv /= u_zoom;

    float2 pan = float2(u_pan.x, -u_pan.y);

    float3 col = lerp(float3(0.01, 0.015, 0.04), float3(0.04, 0.02, 0.09),
                      fragCoord.y / u_resolution.y);

    float2 nebUv = uv * 1.5 + pan * 0.0002 + u_flow_offset * 0.0003
                 + float2(u_time * 0.01, u_time * 0.005);
    float n = fbm(nebUv);
    float n2 = fbm(nebUv * 2.5 - float2(u_time * 0.015, u_time * 0.015));
    col += lerp(float3(0.25, 0.05, 0.35), float3(0.05, 0.20, 0.45), n) * pow(n, 1.8) * 0.5;
    col += float3(0.1, 0.3, 0.4) * pow(n2, 3.0) * 0.25;

    [unroll] for (int i = 1; i <= 3; i++) {
        float fi = float(i);
        float depth = fi * 0.5;
        float2 sp = uv * (45.0 / depth) + pan * 0.001 * depth;
        float2 id = floor(sp);
        float2 fp = frac(sp) - 0.5;
        float h = hash(id);
        if (h > 0.96) {
            float twink = 0.5 + 0.5 * sin(u_time * 1.5 + h * 50.0);
            float dd = length(fp);
            float3 starCol = lerp(float3(0.7, 0.9, 1.0), float3(1.0, 0.85, 0.7), frac(h * 133.7));
            float glow = smoothstep(0.06, 0.0, dd) + smoothstep(0.2, 0.0, dd) * 0.3;
            col += starCol * glow * twink / depth;
        }
    }

    {
        float2 drift = -u_flow_offset * 0.0007 + float2(u_time * 0.12, 0.0);
        float2 p = uv * float2(1.8, 12.0) + drift;
        float2 id = floor(p);
        float2 f = frac(p) - 0.5;
        float h = hash(id);
        if (h > 0.86) {
            float streak = smoothstep(0.5, 0.0, abs(f.y) * 5.0) * smoothstep(0.5, 0.0, abs(f.x) * 1.1);
            col += float3(0.45, 0.65, 1.0) * streak * (h - 0.86) * 3.5;
        }
    }

    col = draw_planet(col, uv, float2(-0.65, 0.30) - pan * 0.00015, 0.07,
                      float3(0.85, 0.85, 0.90), float3(0.18, 0.18, 0.22),
                      normalize(float2(1.0, 0.3)), 0.0);
    col = draw_planet(col, uv, float2(0.70, 0.15) - pan * 0.00030, 0.13,
                      float3(0.35, 0.65, 0.55), float3(0.08, 0.15, 0.12),
                      normalize(float2(-0.6, 0.4)), 0.0);
    col = draw_planet(col, uv, float2(-0.40, -0.30) - pan * 0.00055, 0.22,
                      float3(0.90, 0.60, 0.35), float3(0.15, 0.05, 0.08),
                      normalize(float2(0.7, 0.5)), 15.0);

    // ---------------- LOCK SCREEN TRANSITION ----------------
    float L = clamp(u_lock_amount, 0.0, 1.0);
    L = L * L * (3.0 - 2.0 * L);
    if (L > 0.001) {
        float drift = u_time * 0.003;
        float3 lcol = lerp(float3(0.004, 0.006, 0.018), float3(0.010, 0.015, 0.040),
                           clamp(uv.y * 0.5 + 0.5, 0.0, 1.0));
        float bandAxis = dot(uv, normalize(float2(0.6, -0.8))) + 0.55;
        float bandShape = exp(-bandAxis * bandAxis * 4.0);
        float bandTex = fbm(uv * 1.3 + float2(drift, -2.0));
        lcol += lerp(float3(0.04, 0.05, 0.09), float3(0.07, 0.06, 0.11), bandTex)
                * bandShape * bandTex * 0.5;
        [unroll] for (int i = 1; i <= 2; i++) {
            float dens = (i == 1) ? 55.0 : 95.0;
            float thr = (i == 1) ? 0.980 : 0.992;
            float2 sp = uv * dens + pan * 0.0002 * float(i);
            float2 id = floor(sp);
            float2 fp = frac(sp) - 0.5;
            float h = hash(id);
            if (h > thr) {
                float dd = length(fp);
                float core = smoothstep(0.12, 0.0, dd);
                float3 sc = (i == 2) ? float3(0.45, 0.30, 0.26) : float3(0.45, 0.52, 0.68);
                lcol += sc * core * ((i == 2) ? 0.35 : 0.60);
            }
        }
        lcol += float3(0.16, 0.15, 0.21) * galaxy(uv, float2(0.52, 0.34) + drift, 0.6, float2(0.13, 0.045)) * 0.55;
        lcol += float3(0.13, 0.13, 0.19) * galaxy(uv, float2(-0.58, -0.22) + drift, -0.3, float2(0.09, 0.030)) * 0.45;
        lcol += float3(0.12, 0.11, 0.17) * galaxy(uv, float2(0.05, -0.40) + drift, 1.2, float2(0.05, 0.020)) * 0.40;
        float vig = smoothstep(1.25, 0.15, length(uv));
        lcol *= lerp(0.30, 1.0, vig);
        col = lerp(col, lcol, L);
    }

    return float4(col, 1.0) * alpha * 0.75;
}
