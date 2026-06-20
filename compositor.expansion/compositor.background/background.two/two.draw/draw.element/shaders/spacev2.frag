precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

float hash(vec2 p){ return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453); }
float noise(vec2 p){
    vec2 i=floor(p), f=fract(p);
    f=f*f*(3.0-2.0*f);
    return mix(mix(hash(i),hash(i+vec2(1,0)),f.x),
               mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y);
}
float fbm(vec2 p){
    float v=0., a=0.5;
    for(int i=0;i<5;i++){ v+=a*noise(p); p*=2.0; a*=0.5; }
    return v;
}

float sdCircle(vec2 p, float r){ return length(p)-r; }

// Draw a planet with terminator, color, and a soft atmospheric rim
vec3 draw_planet(vec3 col, vec2 uv, vec2 center, float radius,
                 vec3 lightSide, vec3 darkSide, vec2 lightDir, float bandFreq) {
    vec2 pp = uv - center;
    float d = sdCircle(pp, radius);
    float mask = smoothstep(0.004, -0.004, d);

    if (mask <= 0.0) return col;

    // Core lighting
    float lit = smoothstep(-radius*0.6, radius*0.6, dot(pp, lightDir));
    vec3 base = mix(darkSide, lightSide, lit);

    // Organic bands for gas giants
    if (bandFreq > 0.0) {
        float band = sin(pp.y * bandFreq + center.x*3.0) * 0.5 + 0.5;
        float bandNoise = fbm(pp * 15.0) * 0.15;
        base = mix(base, base * 0.75, smoothstep(0.2, 0.8, band + bandNoise));
    }

    // Atmospheric rim lighting (Fresnel effect on lit edge)
    float rim = smoothstep(radius * 0.5, radius, length(pp));
    float rimLit = smoothstep(-radius * 0.2, radius, dot(pp, lightDir));
    vec3 atmosphere = lightSide * rim * rimLit * 0.5;

    return mix(col, base + atmosphere, mask);
}

void main() {
    vec2 uv = (gl_FragCoord.xy - 0.5*u_resolution) / u_resolution.y;
    uv /= u_zoom;

    // Standardize UI vs GLSL coordinate mapping
    vec2 pan = vec2(u_pan.x, -u_pan.y);

    // Richer background gradient
    vec3 col = mix(vec3(0.01, 0.015, 0.04), vec3(0.04, 0.02, 0.09), gl_FragCoord.y/u_resolution.y);

    // Nebula - Dual layered for depth
    vec2 nebUv = uv*1.5 + pan*0.0002 + u_flow_offset*0.0003 + vec2(u_time*0.01, u_time*0.005);
    float n = fbm(nebUv);
    float n2 = fbm(nebUv * 2.5 - vec2(u_time * 0.015));
    col += mix(vec3(0.25, 0.05, 0.35), vec3(0.05, 0.20, 0.45), n) * pow(n, 1.8) * 0.5;
    col += vec3(0.1, 0.3, 0.4) * pow(n2, 3.0) * 0.25;

    // Starfields - Color variance and soft glow
    for(int i=1; i<=3; i++){
        float fi = float(i);
        float depth = fi * 0.5;
        vec2 sp = uv * (45.0/depth) + pan * 0.001 * depth;
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash(id);

        if (h > 0.96) {
            float twink = 0.5 + 0.5*sin(u_time*1.5 + h*50.0);
            float d = length(fp);

            // Randomize star color (blue to warm white)
            vec3 starCol = mix(vec3(0.7, 0.9, 1.0), vec3(1.0, 0.85, 0.7), fract(h * 133.7));

            // Core + outer glow
            float glow = smoothstep(0.06, 0.0, d) + smoothstep(0.2, 0.0, d) * 0.3;
            col += starCol * glow * twink / depth;
        }
    }

    // Solar wind streaks - Softened edges
    {
        vec2 drift = -u_flow_offset * 0.0007 + vec2(u_time*0.12, 0.0);
        vec2 p = uv * vec2(1.8, 12.0) + drift;
        vec2 id = floor(p);
        vec2 f  = fract(p) - 0.5;
        float h = hash(id);
        if (h > 0.86) {
            float streak = smoothstep(0.5, 0.0, abs(f.y)*5.0) * smoothstep(0.5, 0.0, abs(f.x)*1.1);
            col += vec3(0.45, 0.65, 1.0) * streak * (h - 0.86) * 3.5;
        }
    }

    // ---- Planets ----
    // Far moon (top-left, slow parallax)
    col = draw_planet(col, uv,
        vec2(-0.65, 0.30) - pan*0.00015, 0.07,
        vec3(0.85, 0.85, 0.90), vec3(0.18, 0.18, 0.22),
        normalize(vec2(1.0, 0.3)), 0.0);

    // Mid-distance terrestrial (right side)
    col = draw_planet(col, uv,
        vec2(0.70, 0.15) - pan*0.00030, 0.13,
        vec3(0.35, 0.65, 0.55), vec3(0.08, 0.15, 0.12),
        normalize(vec2(-0.6, 0.4)), 0.0);

    // Foreground gas giant (bottom-left, biggest parallax)
    col = draw_planet(col, uv,
        vec2(-0.40, -0.30) - pan*0.00055, 0.22,
        vec3(0.90, 0.60, 0.35), vec3(0.15, 0.05, 0.08),
        normalize(vec2(0.7, 0.5)), 15.0);

    gl_FragColor = vec4(col, 1.0) * alpha * 0.75;
}