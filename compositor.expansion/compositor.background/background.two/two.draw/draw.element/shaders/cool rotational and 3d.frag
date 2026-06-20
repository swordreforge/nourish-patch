precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// ==========================================
// 1. Math, Hashes & 3D Noise Suite
// ==========================================
#define MAX_STEPS 80
#define MAX_DIST 50.0
#define SURF_DIST 0.005

mat2 rot(float a) {
    float s = sin(a), c = cos(a);
    return mat2(c, -s, s, c);
}

float hash11(float p) {
    p = fract(p * .1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

float hash31(vec3 p3) {
    p3  = fract(p3 * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec3 hash33(vec3 p) {
    p = vec3(dot(p, vec3(127.1, 311.7, 74.7)),
             dot(p, vec3(269.5, 183.3, 246.1)),
             dot(p, vec3(113.5, 271.9, 124.6)));
    return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

// High-quality 3D Value Noise
float noise3D(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    
    return mix(mix(mix(hash31(i + vec3(0,0,0)), hash31(i + vec3(1,0,0)), f.x),
                   mix(hash31(i + vec3(0,1,0)), hash31(i + vec3(1,1,0)), f.x), f.y),
               mix(mix(hash31(i + vec3(0,0,1)), hash31(i + vec3(1,0,1)), f.x),
                   mix(hash31(i + vec3(0,1,1)), hash31(i + vec3(1,1,1)), f.x), f.y), f.z);
}

// 3D Fractional Brownian Motion for terrain/clouds
float fbm(vec3 p) {
    float f = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 5; i++) {
        f += amp * noise3D(p);
        p *= 2.01;
        amp *= 0.5;
    }
    return f;
}

// Ridged multifractal for cracked ice/mountains
float ridge3D(vec3 p) {
    float f = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 5; i++) {
        float n = 1.0 - abs(noise3D(p) * 2.0 - 1.0);
        f += amp * (n * n);
        p *= 2.1;
        amp *= 0.5;
    }
    return f;
}

// ==========================================
// 2. Analytical Intersections (Planets)
// ==========================================
// Ray-Sphere intersection. Returns vec2(near_dist, far_dist)
vec2 iSphere(vec3 ro, vec3 rd, vec4 sph) {
    vec3 oc = ro - sph.xyz;
    float b = dot(oc, rd);
    float c = dot(oc, oc) - sph.w * sph.w;
    float h = b * b - c;
    if (h < 0.0) return vec2(-1.0);
    h = sqrt(h);
    return vec2(-b - h, -b + h);
}

// Planet surface normal with intense displacement mapping
vec3 getPlanetNormal(vec3 p, vec3 center, float radius) {
    vec3 n = normalize(p - center);
    // Create a localized tangent space for the bump map
    vec2 e = vec2(0.01, 0.0);
    vec3 samplePos = p * 2.5; // Scale of continents/ice sheets
    
    // Evaluate 3D terrain height
    float d1 = ridge3D(samplePos + e.xyy) - ridge3D(samplePos - e.xyy);
    float d2 = ridge3D(samplePos + e.yxy) - ridge3D(samplePos - e.yxy);
    float d3 = ridge3D(samplePos + e.yyx) - ridge3D(samplePos - e.yyx);
    
    return normalize(n - vec3(d1, d2, d3) * 0.15); // Bump strength
}

vec3 renderPlanets(vec3 ro, vec3 rd, vec3 globalLight) {
    vec3 col = vec3(0.0);
    float depth = 9999.0;
    
    // Planet Data: x,y,z, radius
    vec4 p1 = vec4(-4.0, 2.0, 15.0, 2.5);  // Distant left
    vec4 p2 = vec4(5.0, -1.5, 20.0, 4.0);  // Massive right
    vec4 p3 = vec4(-2.0, -3.0, 10.0, 1.8); // Foreground left
    
    vec4 planets[3];
    planets[0] = p1; planets[1] = p2; planets[2] = p3;
    
    for (int i = 0; i < 3; i++) {
        vec2 t = iSphere(ro, rd, planets[i]);
        if (t.x > 0.0 && t.x < depth) {
            depth = t.x;
            vec3 pos = ro + rd * t.x;
            vec3 n = getPlanetNormal(pos, planets[i].xyz, planets[i].w);
            
            // Core lighting
            float diff = max(dot(n, globalLight), 0.0);
            float rim = 1.0 - max(dot(n, -rd), 0.0);
            rim = smoothstep(0.6, 1.0, rim);
            
            // Ice base colors
            float iceMix = ridge3D(pos * 3.0);
            vec3 albedo = mix(vec3(0.1, 0.25, 0.4), vec3(0.8, 0.9, 1.0), smoothstep(0.2, 0.7, iceMix));
            
            vec3 planetCol = albedo * (diff + 0.05) + vec3(0.5, 0.7, 1.0) * pow(rim, 4.0) * diff * 2.0;
            col = planetCol;
        }
    }
    
    // Atmospheric halos (Additive blend over distance)
    for (int i = 0; i < 3; i++) {
        vec3 oc = ro - planets[i].xyz;
        float b = dot(oc, rd);
        float c = dot(oc, oc);
        float distToCenter = sqrt(max(0.0, c - b*b));
        float radius = planets[i].w;
        
        if (distToCenter < radius * 1.8 && depth > ( -b )) {
            float halo = smoothstep(radius * 1.8, radius * 0.9, distToCenter);
            float litHalo = max(0.0, dot(normalize(planets[i].xyz - ro), globalLight));
            col += vec3(0.2, 0.4, 0.7) * pow(halo, 3.0) * (litHalo + 0.2) * 0.6;
        }
    }
    
    return col;
}

// ==========================================
// 3. 3D SDF Raymarching (Ice Crystals)
// ==========================================
// Faceted Octahedron for ice shards
float sdOctahedron(vec3 p, float s) {
    p = abs(p);
    return (p.x + p.y + p.z - s) * 0.57735027;
}

// The core distance field map for foreground geometry
vec2 mapCrystals(vec3 p) {
    // Infinite modulo grid for panning over vast distances
    vec3 c = vec3(12.0, 12.0, 8.0);
    vec3 id = floor((p + c * 0.5) / c);
    vec3 q = mod(p + c * 0.5, c) - c * 0.5;
    
    // Deterministic randomness per grid cell
    float h = hash31(id);
    
    if (h > 0.6) { // Sparsity - only spawn crystals in 40% of cells
        // Local rotation based on ID and time
        q.xy *= rot(u_time * 0.2 + h * 6.28);
        q.xz *= rot(u_time * 0.3 + h * 3.14);
        
        // Stretch and deform to make jagged shards rather than perfect shapes
        q.y *= 0.4 + h * 0.5; 
        
        float d = sdOctahedron(q, 1.5 + h * 1.0);
        return vec2(d, 1.0); // 1.0 is material ID for glass/ice
    }
    
    return vec2(MAX_DIST, 0.0);
}

// Standard raymarcher for SDFs
vec2 raymarch(vec3 ro, vec3 rd) {
    float dO = 0.0;
    float mat = 0.0;
    for(int i = 0; i < 60; i++) {
        vec3 p = ro + rd * dO;
        vec2 dS = mapCrystals(p);
        if(dS.x < SURF_DIST) {
            mat = dS.y;
            break;
        }
        dO += dS.x;
        if(dO > MAX_DIST) break;
    }
    return vec2(dO, mat);
}

// SDF Normal
vec3 getNormal(vec3 p) {
    vec2 e = vec2(0.01, 0.0);
    vec3 n = vec3(
        mapCrystals(p + e.xyy).x - mapCrystals(p - e.xyy).x,
        mapCrystals(p + e.yxy).x - mapCrystals(p - e.yxy).x,
        mapCrystals(p + e.yyx).x - mapCrystals(p - e.yyx).x
    );
    return normalize(n);
}

// ==========================================
// 4. Volumetric Clouds (Raymarched Density)
// ==========================================
vec4 renderClouds(vec3 ro, vec3 rd, vec3 lightDir, float maxDepth) {
    vec4 sum = vec4(0.0);
    float t = 0.0;
    
    // Step size and cloud scale
    float stepSize = 0.4;
    
    for (int i = 0; i < 25; i++) {
        vec3 pos = ro + rd * t;
        
        // Only render clouds in the midground to avoid intersecting the camera violently
        if (pos.z > 2.0 && pos.z < 25.0 && t < maxDepth) {
            // Domain warped noise for thick, billowing fog
            vec3 q = pos * 0.2 - vec3(u_time * 0.2, 0.0, 0.0);
            float den = fbm(q);
            den = smoothstep(0.4, 0.8, den); // Thresholding for defined cloud shapes
            
            if (den > 0.01) {
                // Approximate lighting/shadows within the cloud by sampling towards the light
                float shadowDen = fbm(q + lightDir * 0.5);
                float lightAtten = exp(-shadowDen * 3.0);
                
                vec3 col = mix(vec3(0.1, 0.2, 0.3), vec3(0.8, 0.9, 1.0), lightAtten);
                
                // Front-to-back alpha blending
                col *= den * 0.5;
                sum += vec4(col, den) * (1.0 - sum.a);
                
                if (sum.a > 0.95) break; // Early exit if fully opaque
            }
        }
        t += stepSize;
    }
    return sum;
}

// ==========================================
// Main Pipeline
// ==========================================
void main() {
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;
    uv /= u_zoom;
    
    // Scale pan and flow mapping for realistic 3D translation
    vec3 panOffset = vec3(u_pan.x, -u_pan.y, 0.0) * 0.005;
    vec3 windOffset = vec3(u_flow_offset.x, 0.0, u_flow_offset.y) * 0.002;
    
    // ----------------------------------------------------
    // Camera Setup
    // ----------------------------------------------------
    // Origin moves physically through the 3D space based on UI pan/flow
    vec3 ro = vec3(0.0, 0.0, -5.0) + panOffset + windOffset;
    vec3 rd = normalize(vec3(uv, 1.0)); // Look forward
    
    // Add slight camera tilt based on pan velocity to simulate physical movement
    rd.xy *= rot(panOffset.x * 0.02);
    
    vec3 globalLight = normalize(vec3(0.8, 0.7, -0.5));
    
    // ----------------------------------------------------
    // LAYER 1: Deep Cosmic Background & Dense Starfield
    // ----------------------------------------------------
    vec3 col = mix(vec3(0.02, 0.03, 0.08), vec3(0.08, 0.12, 0.2), gl_FragCoord.y / u_resolution.y);
    
    // Projected 3D starfield (mapped to a sphere at infinity)
    vec3 starRay = normalize(rd);
    float starFBM = noise3D(starRay * 200.0);
    float stars = smoothstep(0.85, 1.0, starFBM);
    stars *= 0.5 + 0.5 * sin(u_time * 5.0 + starRay.x * 100.0); // Twinkle
    col += mix(vec3(0.5, 0.7, 1.0), vec3(1.0), hash31(starRay)) * stars * 2.0;

    // ----------------------------------------------------
    // LAYER 2: Raymarched Planets (Mid to Background)
    // ----------------------------------------------------
    vec3 planetCol = renderPlanets(ro, rd, globalLight);
    if (length(planetCol) > 0.0) {
        col = planetCol;
    }

    // ----------------------------------------------------
    // LAYER 3: Volumetric Clouds
    // ----------------------------------------------------
    // Determine the max depth so clouds don't render in front of foreground crystals
    vec2 shardData = raymarch(ro, rd);
    float maxCloudDepth = (shardData.x < MAX_DIST) ? shardData.x : MAX_DIST;
    
    vec4 clouds = renderClouds(ro, rd, globalLight, maxCloudDepth);
    col = mix(col, clouds.rgb, clouds.a);

    // ----------------------------------------------------
    // LAYER 4: High-Velocity Cinematic Blizzard
    // ----------------------------------------------------
    // Using 2D projection for fast, dense blizzard overlay
    vec2 snowDrift = vec2(-u_time * 0.8, u_time * 0.6) - windOffset.xy * 2.0;
    for (float i = 1.0; i <= 3.0; i++) {
        vec2 sp = uv * (25.0 / i) + panOffset.xy * 2.0 * i + snowDrift * (1.0 / i);
        sp.x += sin(sp.y * 5.0 + u_time) * 0.05; 
        
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash11(id.x + id.y * 31.1 + i * 11.3);
        
        if (h > 0.65) {
            float blur = 0.05 + i * 0.01;
            float mask = smoothstep(blur, 0.0, length(fp));
            
            // Simulating motion blur on the falling snow
            float motionBlur = smoothstep(0.2, 0.0, abs(fp.x + fp.y)); 
            col += vec3(0.9, 0.95, 1.0) * mask * motionBlur * (h * 0.8 + 0.2);
        }
    }

    // ----------------------------------------------------
    // LAYER 5: Macro Refractive Ice Shards (Foreground)
    // ----------------------------------------------------
    if (shardData.x < MAX_DIST) {
        vec3 p = ro + rd * shardData.x;
        vec3 n = getNormal(p);
        
        // True physical Refraction vector
        vec3 refRay = refract(rd, n, 0.7); // IOR for ice approx
        
        // Fake the background sampling behind the crystal by shifting the original UV 
        // based on the refraction normal, and sampling the 2D background color logic
        vec2 refUv = uv + refRay.xy * 0.2;
        
        // Base glass lighting
        float fresnel = pow(1.0 - max(dot(n, -rd), 0.0), 3.0);
        float diff = max(dot(n, globalLight), 0.0);
        float spec = pow(max(dot(reflect(rd, n), globalLight), 0.0), 32.0);
        
        // Inner facets to simulate thick, cracked ice
        float innerDisp = ridge3D(p * 5.0);
        
        // Combine refraction with specular highlights and deep ice scattering
        vec3 crystalCol = mix(col, vec3(0.4, 0.7, 0.9), 0.3); // Tint background
        crystalCol += vec3(0.2, 0.5, 0.8) * diff * innerDisp; // Deep scatter
        crystalCol += vec3(1.0) * spec * 2.0;                 // Sharp glint
        crystalCol += vec3(0.8, 0.9, 1.0) * fresnel * 1.5;    // Edge glow
        
        // Cinematic Depth of Field: Shards extremely close to the camera get heavily blurred
        float dof = smoothstep(2.0, 0.0, shardData.x);
        
        col = mix(crystalCol, col, dof * 0.7); // Blend back to raw background if out of focus
    }

    // Vignette and final color grading
    uv *= 1.0 - uv.yx;
    float vig = uv.x * uv.y * 15.0;
    col *= pow(vig, 0.15);
    
    // Contrast boost
    col = smoothstep(0.0, 1.2, col);

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0) * alpha;
}