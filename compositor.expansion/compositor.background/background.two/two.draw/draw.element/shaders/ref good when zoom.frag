precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// ==========================================
// Noise & Hash Suite
// ==========================================
vec2 hash22(vec2 p) {
    vec3 p3 = fract(vec3(p.xyx) * vec3(.1031, .1030, .0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy) * 2.0 - 1.0;
}

float hash12(vec2 p) {
    vec3 p3  = fract(vec3(p.xyx) * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = dot(hash22(i + vec2(0.0, 0.0)), f - vec2(0.0, 0.0));
    float b = dot(hash22(i + vec2(1.0, 0.0)), f - vec2(1.0, 0.0));
    float c = dot(hash22(i + vec2(0.0, 1.0)), f - vec2(0.0, 1.0));
    float d = dot(hash22(i + vec2(1.0, 1.0)), f - vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 0.5 + 0.5;
}

float fbm(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    mat2 rot = mat2(0.866, -0.5, 0.5, 0.866);
    for (int i = 0; i < 6; i++) {
        v += a * noise(p);
        p = rot * p * 2.0;
        a *= 0.5;
    }
    return v;
}

// Ridged noise for sharp ice/mountains
float ridge(vec2 p) {
    float v = 0.0;
    float a = 0.5;
    for (int i = 0; i < 5; i++) {
        float n = 1.0 - abs(noise(p) * 2.0 - 1.0);
        v += a * (n * n);
        p *= 2.0;
        a *= 0.5;
    }
    return v;
}

// ==========================================
// Geometric SDFs
// ==========================================
float sdHex(vec2 p, float r) {
    const vec3 k = vec3(-0.866025404, 0.5, 0.577350269);
    p = abs(p);
    p -= 2.0 * min(dot(k.xy, p), 0.0) * k.xy;
    p -= vec2(clamp(p.x, -k.z * r, k.z * r), r);
    return length(p) * sign(p.y);
}

// ==========================================
// Rendering Modules
// ==========================================
vec3 render_planet(vec3 bgCol, vec2 uv, vec2 center, float radius, vec3 colorDark, vec3 colorLight, vec2 lightDir) {
    vec2 p = uv - center;
    float dist = length(p);
    
    // Smooth edges for anti-aliasing
    float mask = smoothstep(radius + 0.005, radius - 0.005, dist);
    
    // Atmospheric glow (outer)
    float outerGlow = smoothstep(radius * 2.0, radius, dist) * smoothstep(-radius, radius * 1.5, dot(p, lightDir));
    vec3 glowCol = mix(colorDark, vec3(1.0), 0.5) * outerGlow * 0.6;
    
    if (mask <= 0.0) return bgCol + glowCol;

    // Pseudo-3D sphere normal mapping
    float z = sqrt(max(0.0, radius * radius - dist * dist));
    vec3 normal = normalize(vec3(p.x, p.y, z));
    vec3 light3D = normalize(vec3(lightDir.x, lightDir.y, 0.5));
    
    // Diffuse lighting
    float diffuse = max(dot(normal, light3D), 0.0);
    
    // Surface texturing (mapped to the pseudo-normal for spherical wrapping)
    vec2 sphereUv = normal.xy * 3.0 + center * 10.0;
    float surfaceNoise = fbm(sphereUv * 2.0);
    float cracks = ridge(sphereUv * 4.0);
    
    vec3 albedo = mix(colorDark, colorLight, surfaceNoise);
    albedo = mix(albedo, vec3(0.9, 0.95, 1.0), smoothstep(0.4, 0.7, cracks)); // Ice caps/ridges

    // Inner atmospheric rim
    float rim = 1.0 - max(dot(normal, vec3(0.0, 0.0, 1.0)), 0.0);
    rim = smoothstep(0.5, 1.0, rim);
    vec3 rimLight = colorLight * pow(rim, 3.0) * diffuse * 2.0;

    vec3 finalCol = albedo * (diffuse + 0.05) + rimLight;
    return mix(bgCol + glowCol, finalCol, mask);
}

void main() {
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;
    uv /= u_zoom;
    
    vec2 pan = vec2(u_pan.x, -u_pan.y);
    vec2 flow = u_flow_offset * 0.0003;
    
    // ----------------------------------------------------
    // 1. Deep Cosmic Background & Volumetric Fog
    // ----------------------------------------------------
    vec3 col = mix(vec3(0.03, 0.05, 0.1), vec3(0.1, 0.15, 0.25), gl_FragCoord.y / u_resolution.y);
    
    vec2 fogUv = uv * 1.5 + pan * 0.0001 - flow * 0.5;
    
    // Domain warped clouds
    vec2 q = vec2(fbm(fogUv), fbm(fogUv + vec2(5.2, 1.3)));
    vec2 r = vec2(fbm(fogUv + 4.0 * q + vec2(1.7, 9.2) + u_time * 0.02),
                  fbm(fogUv + 4.0 * q + vec2(8.3, 2.8) - u_time * 0.015));
    float fog = fbm(fogUv + 4.0 * r);
    
    // Lighten the clouds where the main light source would be (top right)
    float cloudLight = smoothstep(-0.5, 0.5, uv.x + uv.y);
    vec3 cloudCol = mix(vec3(0.1, 0.2, 0.3), vec3(0.6, 0.8, 0.9), fog * cloudLight);
    col += cloudCol * pow(fog, 1.5) * 1.2;

    // ----------------------------------------------------
    // 2. Starfield & Deep Snow Dust
    // ----------------------------------------------------
    for(float i = 1.0; i <= 3.0; i++) {
        vec2 sp = uv * (50.0 - i * 5.0) + pan * 0.0005 * i;
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i);
        
        if (h > 0.92) {
            float size = (h - 0.92) * 12.0;
            float twink = 0.5 + 0.5 * sin(u_time * 2.0 + h * 50.0);
            float glow = smoothstep(0.15 * size, 0.0, length(fp));
            col += mix(vec3(0.5, 0.7, 1.0), vec3(1.0), fract(h * 33.3)) * glow * twink;
        }
    }

    // ----------------------------------------------------
    // 3. Icy Planets (Back to Front)
    // ----------------------------------------------------
    vec2 globalLight = normalize(vec2(0.8, 0.6));
    
    // Distant background planet (Top Right)
    col = render_planet(col, uv, vec2(0.65, 0.35) - pan * 0.0002, 0.08, 
                        vec3(0.15, 0.25, 0.4), vec3(0.6, 0.8, 0.9), globalLight);
                        
    // Midground fractured ice planet (Center Left)
    col = render_planet(col, uv, vec2(-0.45, 0.15) - pan * 0.0004, 0.18, 
                        vec3(0.1, 0.2, 0.3), vec3(0.8, 0.9, 1.0), globalLight);
                        
    // Massive foreground ice giant (Bottom Right corner cut-off)
    col = render_planet(col, uv, vec2(0.55, -0.4) - pan * 0.0007, 0.35, 
                        vec3(0.2, 0.4, 0.6), vec3(0.9, 0.95, 1.0), globalLight);

    // ----------------------------------------------------
    // 4. Snowy Mountain Peaks (Bottom Right Anchor)
    // ----------------------------------------------------
    vec2 mntUv = uv + vec2(-0.3, 0.4) + pan * 0.001;
    if (mntUv.x > 0.0) {
        float peakHeight = ridge(vec2(mntUv.x * 3.0, 0.0)) * 0.3 - 0.2;
        float mntMask = smoothstep(0.01, -0.01, mntUv.y - peakHeight);
        
        if (mntMask > 0.0) {
            float mntNoise = fbm(mntUv * 10.0);
            float mntLighting = smoothstep(-0.2, 0.2, ridge(mntUv * 15.0 + vec2(0.1, 0.0)) - ridge(mntUv * 15.0));
            vec3 mntCol = mix(vec3(0.15, 0.2, 0.3), vec3(0.85, 0.9, 1.0), mntLighting);
            col = mix(col, mntCol, mntMask * smoothstep(0.0, 0.2, mntUv.x)); // Fade in from left
        }
    }

    // ----------------------------------------------------
    // 5. High-Velocity Blizzard
    // ----------------------------------------------------
    vec2 wind = vec2(-u_time * 0.4, u_time * 0.3) - flow * 1.5;
    for (float i = 1.0; i <= 4.0; i++) {
        vec2 sp = uv * (30.0 / i) + pan * 0.002 * i + wind * (1.0 / i);
        sp.x += sin(sp.y * 3.0 + u_time) * 0.1; // Wind swirl
        
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i * 15.0);
        
        if (h > 0.75) {
            float blur = 0.03 + i * 0.015;
            float snowMask = smoothstep(blur, 0.0, length(fp));
            col += vec3(0.8, 0.9, 1.0) * snowMask * (h * 0.8 + 0.2);
        }
    }

    // ----------------------------------------------------
    // 6. Macro Refractive Ice Shards
    // ----------------------------------------------------
    vec2 shardUv = uv * 2.5 + pan * 0.004 + wind * 0.8;
    vec2 sId = floor(shardUv);
    vec2 sFp = fract(shardUv) - 0.5;
    float sH = hash12(sId + 42.0);
    
    if (sH > 0.88) {
        // Randomly rotate each shard
        float angle = sH * 6.28 + u_time * 0.2 * (sH > 0.95 ? 1.0 : -1.0);
        float s = sin(angle), c = cos(angle);
        mat2 rot = mat2(c, -s, s, c);
        vec2 p = rot * sFp;
        
        // Stretch the hexagon to make it look like a broken shard
        p.y *= 0.5 + sH * 0.5;
        
        float size = 0.15 + sH * 0.15;
        float dist = sdHex(p, size);
        
        // Out of focus depth of field
        float mask = smoothstep(0.12, -0.05, dist);
        
        if (mask > 0.0) {
            // Fake internal volume/facets for refraction
            float inner = sdHex(p, size * 0.6);
            float facet = ridge(p * 10.0);
            
            // Refraction: Brighten and tint the existing background
            vec3 shardCol = col * 1.4; 
            
            // Edge rim light and core specular glints
            shardCol += vec3(0.4, 0.7, 1.0) * smoothstep(0.05, -0.05, inner) * 0.5;
            shardCol += vec3(1.0) * pow(max(0.0, 1.0 - abs(inner) * 15.0), 3.0) * (facet * 1.5);
            
            // Add a slight chromatic aberration at the heavily blurred edges
            shardCol.r += 0.1 * smoothstep(0.0, 0.1, dist);
            shardCol.b += 0.2 * smoothstep(0.1, 0.0, dist);
            
            col = mix(col, shardCol, mask * 0.9);
        }
    }

    // Vignette for cinematic framing
    float vignette = 1.0 - smoothstep(0.5, 1.5, length(uv));
    col *= mix(vec3(0.5, 0.6, 0.8), vec3(1.0), vignette);

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0) * alpha;
}