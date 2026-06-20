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

vec2 curl(vec2 p) {
    float e = 0.01;
    float n1 = fbm(p + vec2(0.0,  e));
    float n2 = fbm(p - vec2(0.0,  e));
    float n3 = fbm(p + vec2(e,  0.0));
    float n4 = fbm(p - vec2(e,  0.0));
    return vec2(n1 - n2, -(n3 - n4)) / (2.0*e);
}

void main() {
    vec2 uv = (gl_FragCoord.xy - 0.5*u_resolution) / u_resolution.y;
    uv /= u_zoom;

    vec2 worldUv = uv + u_pan * 0.0005;

    vec3 col = mix(vec3(0.02, 0.03, 0.08), vec3(0.05, 0.08, 0.18), 0.5 + uv.y);

    // Flow_offset adds a directional bias to the entire field — panning
    // genuinely "pushes" the streamlines in that direction
    vec2 flowBias = u_flow_offset * 0.0004;

    vec2 p = worldUv;
    float intensity = 0.0;
    const int STEPS = 12;
    for (int i = 0; i < STEPS; i++) {
        vec2 v = curl(p * 1.5 + u_time * 0.05) + flowBias;
        p += v * 0.02;
        float lineX = abs(fract(p.x * 6.0) - 0.5);
        float lineY = abs(fract(p.y * 6.0) - 0.5);
        float line = min(lineX, lineY);
        intensity += smoothstep(0.03, 0.0, line) * (1.0 / float(STEPS));
    }

    vec3 flowColA = vec3(0.20, 0.60, 0.90);
    vec3 flowColB = vec3(0.85, 0.30, 0.70);
    float t = 0.5 + 0.5*sin(u_time*0.2 + worldUv.x*0.5);
    vec3 flowCol = mix(flowColA, flowColB, t);

    col += flowCol * intensity * 0.6;

    // Glowing particles that ride the field and drift with flow_offset
    for (int i = 1; i <= 3; i++) {
        float fi = float(i);
        float depth = fi * 0.5;
        vec2 pUv = worldUv * (4.0/depth) - u_flow_offset * 0.0008 / depth
                                         + u_time * 0.05 * vec2(0.0, -1.0) / depth;
        vec2 id = floor(pUv);
        vec2 f  = fract(pUv) - 0.5;
        float h = hash(id);
        if (h > 0.93) {
            float d = length(f);
            float pulse = 0.5 + 0.5*sin(u_time*1.5 + h*20.0);
            col += flowCol * smoothstep(0.15, 0.0, d) * pulse * 0.4 / depth;
        }
    }

    col *= mix(0.5, 1.0, smoothstep(1.4, 0.3, length(uv)));

    gl_FragColor = vec4(col, 1.0) * alpha * 0.8;
}