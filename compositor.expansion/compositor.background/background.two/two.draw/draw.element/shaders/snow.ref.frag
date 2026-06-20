precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// ==========================================
// 1. Heavy Noise & Hashing Suite
// ==========================================

vec2 hash22(vec2 p) {
    p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
    return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

float hash12(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float noise2D(vec2 p) {
    const float K1 = 0.366025404; // (sqrt(3)-1)/2;
    const float K2 = 0.211324865; // (3-sqrt(3))/6;
    vec2 i = floor(p + (p.x + p.y) * K1);
    vec2 a = p - i + (i.x + i.y) * K2;
    float m = step(a.y, a.x);
    vec2 o = vec2(m, 1.0 - m);
    vec2 b = a - o + K2;
    vec2 c = a - 1.0 + 2.0 * K2;
    vec3 h = max(0.5 - vec3(dot(a, a), dot(b, b), dot(c, c)), 0.0);
    vec3 n = h * h * h * h * vec3(dot(a, hash22(i + 0.0)), dot(b, hash22(i + o)), dot(c, hash22(i + 1.0)));
    return dot(n, vec3(70.0));
}

// Standard FBM for planets and dust
float fbm(vec2 p) {
    float f = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 6; i++) {
        f += amp * noise2D(p);
        p *= 2.0;
        amp *= 0.5;
    }
    return f * 0.5 + 0.5;
}

// Ridged multifractal for icy terrain and cracks
float ridge_fbm(vec2 p) {
    float f = 0.0;
    float amp = 0.5;
    float weight = 1.0;
    for (int i = 0; i < 5; i++) {
        float n = 1.0 - abs(noise2D(p));
        n *= n;
        f += amp * n * weight;
        weight = max(min(n * 2.0, 1.0), 0.0);
        p *= 2.2;
        amp *= 0.5;
    }
    return f;
}

// Domain warped FBM for thick, volumetric-looking fog
float fog_fbm(vec2 p) {
    vec2 q = vec2(fbm(p + vec2(0.0, 0.0)), fbm(p + vec2(5.2, 1.3)));
    vec2 r = vec2(fbm(p + 4.0 * q + vec2(1.7, 9.2) + u_time * 0.05),
                  fbm(p + 4.0 * q + vec2(8.3, 2.8) - u_time * 0.04));
    return fbm(p + 4.0 * r);
}

// ==========================================
// 2. Geometry & Shape SDFs
// ==========================================

// Sharp, faceted crystal SDF
float sdCrystal(vec2 p, float size) {
    p = abs(p);
    float d1 = dot(p, normalize(vec2(1.0, 2.5))) - size;
    float d2 = dot(p, normalize(vec2(2.5, 0.5))) - size * 0.7;
    return max(d1, d2);
}

// Mountain terrain height function
float terrain_height(float x, float parallax) {
    float h = 0.0;
    h += ridge_fbm(vec2(x * 2.0 + parallax, 0.0)) * 0.4;
    h += fbm(vec2(x * 5.0 + parallax * 1.5, 10.0)) * 0.1;
    return h - 0.5; // Offset to bottom of screen
}

// ==========================================
// 3. Rendering Modules
// ==========================================

vec3 render_planet(vec3 bgCol, vec2 uv, vec2 center, float radius, vec3 baseCol, vec3 iceCol, vec2 lightDir) {
    vec2 p = uv - center;
    float dist = length(p);
    
    // Core planet mask
    float mask = smoothstep(0.005, -0.005, dist - radius);
    if (mask <= 0.0) {
        // Atmospheric shroud bleeding into space
        float shroudMask = smoothstep(radius * 1.6, radius, dist);
        float shroudLit = smoothstep(-radius * 0.5, radius * 1.5, dot(p, lightDir));
        vec3 shroud = baseCol * shroudMask * shroudLit * 0.4;
        return bgCol + shroud;
    }

    // Heavy 2D surface texturing (Ice sheets and continents)
    float surfaceNoise = fbm(p * 10.0 + center * 5.0);
    float crackNoise = ridge_fbm(p * 15.0 - center * 2.0);
    
    vec3 col = mix(baseCol, iceCol, surfaceNoise);
    col = mix(col, vec3(1.0), smoothstep(0.3, 0.8, crackNoise) * 0.5); // Snow caps/ice ridges

    // 2D Lighting (Core shadow)
    float shadowMask = smoothstep(-radius * 0.2, radius * 0.8, dot(p, lightDir));
    col *= mix(vec3(0.05, 0.1, 0.15), vec3(1.0), shadowMask);

    // Inner atmospheric rim light
    float rim = smoothstep(radius * 0.6, radius, dist);
    float rimLit = smoothstep(-radius * 0.1, radius, dot(p, lightDir));
    col += baseCol * rim * rimLit * 0.8;

    return mix(bgCol, col, mask);
}

void main() {
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;
    uv /= u_zoom;
    
    vec2 pan = vec2(u_pan.x, -u_pan.y);
    vec2 wind = u_flow_offset * 0.0003;
    
    // ----------------------------------------------------
    // LAYER 1: Deep Sky & Galactic Dust
    // ----------------------------------------------------
    // Dark, icy gradient from top to bottom
    vec3 col = mix(vec3(0.01, 0.02, 0.05), vec3(0.1, 0.15, 0.25), gl_FragCoord.y / u_resolution.y);
    
    vec2 dustUV = uv * 1.5 + pan * 0.0001;
    float dust = fbm(dustUV);
    col += mix(vec3(0.1, 0.2, 0.3), vec3(0.0), dust) * pow(dust, 2.0) * 0.5;

    // ----------------------------------------------------
    // LAYER 2: Dense Starfield
    // ----------------------------------------------------
    for(int i = 1; i <= 3; i++) {
        float fi = float(i);
        vec2 sp = uv * (60.0 - fi * 10.0) + pan * 0.0005 * fi;
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + fi);
        
        if (h > 0.9) {
            float size = (h - 0.9) * 10.0;
            float twink = 0.5 + 0.5 * sin(u_time * 3.0 + h * 100.0);
            float d = length(fp);
            float glow = smoothstep(0.1 * size, 0.0, d);
            vec3 starCol = mix(vec3(0.7, 0.9, 1.0), vec3(1.0), fract(h * 43.0));
            col += starCol * glow * twink * 0.8;
        }
    }

    // ----------------------------------------------------
    // LAYER 3: Icy Planets in the Sky
    // ----------------------------------------------------
    vec2 lightDir = normalize(vec2(0.8, 0.5));
    
    // Small distant moon
    col = render_planet(col, uv, vec2(-0.6, 0.35) - pan * 0.0002, 0.1, 
                        vec3(0.2, 0.3, 0.5), vec3(0.7, 0.8, 0.9), lightDir);
                        
    // Massive midground ice giant
    col = render_planet(col, uv, vec2(0.5, 0.15) - pan * 0.0004, 0.28, 
                        vec3(0.1, 0.25, 0.4), vec3(0.8, 0.95, 1.0), lightDir);
                        
    // Partially obscured foreground planet
    col = render_planet(col, uv, vec2(-0.4, -0.1) - pan * 0.0007, 0.2, 
                        vec3(0.3, 0.5, 0.7), vec3(0.9, 1.0, 1.0), lightDir);

    // ----------------------------------------------------
    // LAYER 4: Thick Atmospheric Fog
    // ----------------------------------------------------
    vec2 fogUV = uv * 1.2 + pan * 0.001 - wind * 0.5;
    float fog = fog_fbm(fogUV);
    // Fog gets denser near the bottom (terrain level)
    float fogDensity = smoothstep(0.4, -0.6, uv.y) * 1.5; 
    vec3 fogCol = mix(vec3(0.2, 0.3, 0.45), vec3(0.7, 0.85, 1.0), fog);
    col = mix(col, fogCol, fog * fogDensity * 0.8);

    // ----------------------------------------------------
    // LAYER 5: Grounded Terrain (Icy Mountains)
    // ----------------------------------------------------
    float tHeight = terrain_height(uv.x, pan.x * 0.0015);
    // Draw the terrain
    float terrainMask = smoothstep(0.005, -0.005, uv.y - tHeight);
    
    // Terrain texturing
    vec2 terrPos = vec2(uv.x * 4.0 + pan.x * 0.006, uv.y * 4.0);
    float terrDetail = ridge_fbm(terrPos);
    vec3 terrCol = mix(vec3(0.1, 0.15, 0.25), vec3(0.7, 0.85, 1.0), terrDetail);
    
    // Add snow caps to the peaks
    float snowCap = smoothstep(-0.3, 0.0, uv.y) * fbm(terrPos * 2.0);
    terrCol = mix(terrCol, vec3(0.9, 0.95, 1.0), snowCap);
    
    // Blend terrain over the background
    col = mix(col, terrCol, terrainMask);

    // ----------------------------------------------------
    // LAYER 6: Heavy Blizzard Particles
    // ----------------------------------------------------
    vec2 snowDrift = vec2(-u_time * 0.3, u_time * 0.4) - wind * 2.0;
    for (int i = 1; i <= 4; i++) {
        float fi = float(i);
        vec2 sp = uv * (40.0 / fi) + pan * 0.002 * fi + snowDrift * (1.0 / fi);
        
        // Swirling wind effect
        sp.x += sin(sp.y * 2.0 + u_time) * 0.2;
        
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + fi * 10.0);
        
        if (h > 0.6) {
            float d = length(fp);
            // Snowflakes closer to camera are blurrier and larger
            float blur = 0.05 + fi * 0.02;
            float snowMask = smoothstep(blur, 0.0, d);
            col += vec3(0.9, 0.95, 1.0) * snowMask * (h * 0.6 + 0.4);
        }
    }

    // ----------------------------------------------------
    // LAYER 7: Macro Floating Ice Crystals (Immediate Foreground)
    // ----------------------------------------------------
    vec2 shardUv = uv * 3.5 + pan * 0.005 + wind * 1.5;
    vec2 sId = floor(shardUv);
    vec2 sFp = fract(shardUv) - 0.5;
    float sH = hash12(sId + 99.0);
    
    if (sH > 0.85) {
        // Slow random rotation
        float angle = sH * 6.28 + u_time * 0.3 * (sH > 0.9 ? 1.0 : -1.0);
        float s = sin(angle), c = cos(angle);
        mat2 rot = mat2(c, -s, s, c);
        vec2 rotatedFp = rot * sFp;
        
        float size = 0.15 + sH * 0.1;
        float d = sdCrystal(rotatedFp, size);
        
        // Inner facets to fake refraction/thickness
        float inner = sdCrystal(rotatedFp, size * 0.5);
        
        // Out of focus soft mask
        float mask = smoothstep(0.1, -0.05, d);
        
        if (mask > 0.0) {
            // Fake refraction: amplify background color and add icy tint
            vec3 shardCol = col * 1.3;
            // Edge glints
            shardCol += vec3(0.3, 0.6, 0.9) * smoothstep(0.02, -0.05, inner);
            // Core specular flash
            shardCol += vec3(1.0) * pow(max(0.0, 1.0 - abs(inner) * 10.0), 3.0);
            
            col = mix(col, shardCol, mask * 0.85);
        }
    }

    // Final color output
    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0) * alpha;
}