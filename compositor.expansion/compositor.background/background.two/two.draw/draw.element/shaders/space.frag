precision highp float;

// ============================================================================
// [1] UNIFORMS & GLOBAL STATE
// ============================================================================
uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// ============================================================================
// [2] CONSTANTS
// ============================================================================
#define MAX_STEPS 100
#define MAX_DIST 150.0
#define SURF_DIST 0.002
#define PI 3.14159265359
#define TWO_PI 6.28318530718
#define HALF_PI 1.57079632679
#define EPSILON 0.0001

// ============================================================================
// [3] MATRIX & QUATERNION MATH LIBRARY
// ============================================================================
mat2 rot2D(float a) 
{
    float s = sin(a);
    float c = cos(a);
    return mat2(c, -s, s, c);
}

mat3 rotX(float a) 
{
    float s = sin(a);
    float c = cos(a);
    return mat3(
        1.0, 0.0, 0.0,
        0.0, c, -s,
        0.0, s, c
    );
}

mat3 rotY(float a) 
{
    float s = sin(a);
    float c = cos(a);
    return mat3(
        c, 0.0, s,
        0.0, 1.0, 0.0,
        -s, 0.0, c
    );
}

mat3 rotZ(float a) 
{
    float s = sin(a);
    float c = cos(a);
    return mat3(
        c, -s, 0.0,
        s, c, 0.0,
        0.0, 0.0, 1.0
    );
}

vec4 quatInvert(vec4 q) 
{
    return vec4(-q.x, -q.y, -q.z, q.w);
}

vec4 quatMultiply(vec4 q1, vec4 q2) 
{
    return vec4(
        q1.w * q2.x + q1.x * q2.w + q1.y * q2.z - q1.z * q2.y,
        q1.w * q2.y - q1.x * q2.z + q1.y * q2.w + q1.z * q2.x,
        q1.w * q2.z + q1.x * q2.y - q1.y * q2.x + q1.z * q2.w,
        q1.w * q2.w - q1.x * q2.x - q1.y * q2.y - q1.z * q2.z
    );
}

vec3 quatRotate(vec3 p, vec4 q) 
{
    vec3 t = 2.0 * cross(q.xyz, p);
    return p + q.w * t + cross(q.xyz, t);
}

// ============================================================================
// [4] HASHING LIBRARY
// ============================================================================
float hash11(float p) 
{
    p = fract(p * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

float hash12(vec2 p) 
{
    vec3 p3  = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

float hash13(vec3 p3) 
{
    p3  = fract(p3 * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

vec2 hash21(float p) 
{
    vec3 p3 = fract(vec3(p) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

vec2 hash22(vec2 p) 
{
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}

vec3 hash31(float p) 
{
    vec3 p3 = fract(vec3(p) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

vec3 hash32(vec2 p) 
{
    vec3 p3 = fract(vec3(p.xyx) * vec3(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yxz + 33.33);
    return fract((p3.xxy + p3.yzz) * p3.zyx);
}

vec3 hash33(vec3 p) 
{
    p = vec3(dot(p, vec3(127.1, 311.7, 74.7)),
             dot(p, vec3(269.5, 183.3, 246.1)),
             dot(p, vec3(113.5, 271.9, 124.6)));
    return -1.0 + 2.0 * fract(sin(p) * 43758.5453123);
}

// ============================================================================
// [5] PROCEDURAL NOISE LIBRARY
// ============================================================================
float valueNoise2D(vec2 p) 
{
    vec2 i = floor(p);
    vec2 f = fract(p);
    
    vec2 u = f * f * (3.0 - 2.0 * f);
    
    float a = hash12(i + vec2(0.0, 0.0));
    float b = hash12(i + vec2(1.0, 0.0));
    float c = hash12(i + vec2(0.0, 1.0));
    float d = hash12(i + vec2(1.0, 1.0));
    
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float valueNoise3D(vec3 x) 
{
    vec3 i = floor(x);
    vec3 f = fract(x);
    
    vec3 u = f * f * (3.0 - 2.0 * f);
    
    float a = hash13(i + vec3(0.0, 0.0, 0.0));
    float b = hash13(i + vec3(1.0, 0.0, 0.0));
    float c = hash13(i + vec3(0.0, 1.0, 0.0));
    float d = hash13(i + vec3(1.0, 1.0, 0.0));
    float e = hash13(i + vec3(0.0, 0.0, 1.0));
    float g = hash13(i + vec3(1.0, 0.0, 1.0));
    float h = hash13(i + vec3(0.0, 1.0, 1.0));
    float j = hash13(i + vec3(1.0, 1.0, 1.0));
    
    return mix(
        mix(mix(a, b, u.x), mix(c, d, u.x), u.y),
        mix(mix(e, g, u.x), mix(h, j, u.x), u.y), 
        u.z
    );
}

vec3 voronoi3D(vec3 x) 
{
    vec3 n = floor(x);
    vec3 f = fract(x);
    
    vec3 m = vec3(8.0);
    vec3 res = vec3(8.0);
    
    for (int k = -1; k <= 1; k++) 
    {
        for (int j = -1; j <= 1; j++) 
        {
            for (int i = -1; i <= 1; i++) 
            {
                vec3 g = vec3(float(i), float(j), float(k));
                vec3 o = hash33(n + g);
                vec3 r = g + o - f;
                float d = dot(r, r);
                
                if (d < res.x) 
                {
                    res.y = res.x;
                    res.x = d;
                    m = o;
                } 
                else if (d < res.y) 
                {
                    res.y = d;
                }
            }
        }
    }
    return vec3(sqrt(res.x), sqrt(res.y), 0.0);
}

// ============================================================================
// [6] FRACTIONAL BROWNIAN MOTION (FBM) LIBRARY
// ============================================================================
float fbm2D_6(vec2 p) 
{
    float f = 0.0;
    float amp = 0.5;
    mat2 rot = mat2(0.8, 0.6, -0.6, 0.8);
    for (int i = 0; i < 6; i++) 
    {
        f += amp * valueNoise2D(p);
        p = rot * p * 2.0;
        amp *= 0.5;
    }
    return f;
}

float fbm3D_5(vec3 p) 
{
    float f = 0.0;
    float amp = 0.5;
    for (int i = 0; i < 5; i++) 
    {
        f += amp * valueNoise3D(p);
        p *= 2.0;
        amp *= 0.5;
    }
    return f;
}

float ridge3D_4(vec3 p) 
{
    float f = 0.0;
    float amp = 0.5;
    float weight = 1.0;
    for (int i = 0; i < 4; i++) 
    {
        float n = 1.0 - abs(valueNoise3D(p) * 2.0 - 1.0);
        n *= n;
        f += amp * n * weight;
        weight = clamp(n * 2.0, 0.0, 1.0);
        p *= 2.1;
        amp *= 0.5;
    }
    return f;
}

float domainWarp3D(vec3 p, float timeOffset) 
{
    vec3 q = vec3(
        fbm3D_5(p + vec3(0.0, 0.0, 0.0)),
        fbm3D_5(p + vec3(5.2, 1.3, 2.8)),
        fbm3D_5(p + vec3(1.7, 9.2, 4.1))
    );
    
    vec3 r = vec3(
        fbm3D_5(p + 4.0 * q + vec3(1.7, 9.2, 3.4) + timeOffset),
        fbm3D_5(p + 4.0 * q + vec3(8.3, 2.8, 1.2) - timeOffset),
        fbm3D_5(p + 4.0 * q + vec3(3.1, 4.5, 6.7))
    );
    
    return fbm3D_5(p + 4.0 * r);
}

// ============================================================================
// [7] SIGNED DISTANCE FIELD (SDF) PRIMITIVE LIBRARY
// ============================================================================
float sdSphere(vec3 p, float s) 
{
    return length(p) - s;
}

float sdBox(vec3 p, vec3 b) 
{
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}

float sdRoundBox(vec3 p, vec3 b, float r) 
{
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

float sdTorus(vec3 p, vec2 t) 
{
    vec2 q = vec2(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

float sdCylinder(vec3 p, vec3 c) 
{
    return length(p.xz - c.xy) - c.z;
}

float sdCone(vec3 p, vec2 c, float h) 
{
    float q = length(p.xz);
    return max(dot(c.xy, vec2(q, p.y)), -h - p.y);
}

float sdPlane(vec3 p, vec3 n, float h) 
{
    return dot(p, n) + h;
}

float sdHexPrism(vec3 p, vec2 h) 
{
    const vec3 k = vec3(-0.8660254, 0.5, 0.57735);
    p = abs(p);
    p.xy -= 2.0 * min(dot(k.xy, p.xy), 0.0) * k.xy;
    
    vec2 d = vec2(
        length(p.xy - vec2(clamp(p.x, -k.z * h.x, k.z * h.x), h.x)) * sign(p.y - h.x),
        p.z - h.y
    );
    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0));
}

float sdOctahedron(vec3 p, float s) 
{
    p = abs(p);
    return (p.x + p.y + p.z - s) * 0.57735027;
}

float sdCapsule(vec3 p, vec3 a, vec3 b, float r) 
{
    vec3 pa = p - a, ba = b - a;
    float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

// ============================================================================
// [8] SDF BOOLEAN OPERATIONS
// ============================================================================
float opUnion(float d1, float d2) 
{ 
    return min(d1, d2); 
}

float opSubtraction(float d1, float d2) 
{ 
    return max(-d1, d2); 
}

float opIntersection(float d1, float d2) 
{ 
    return max(d1, d2); 
}

float opSmoothUnion(float d1, float d2, float k) 
{
    float h = clamp(0.5 + 0.5 * (d2 - d1) / k, 0.0, 1.0);
    return mix(d2, d1, h) - k * h * (1.0 - h);
}

float opSmoothSubtraction(float d1, float d2, float k) 
{
    float h = clamp(0.5 - 0.5 * (d2 + d1) / k, 0.0, 1.0);
    return mix(d2, -d1, h) + k * h * (1.0 - h);
}

vec3 opRepetition(vec3 p, vec3 c) 
{
    return mod(p + 0.5 * c, c) - 0.5 * c;
}

// ============================================================================
// [9] PBR & MICROFACET BRDF LIGHTING MODEL
// ============================================================================
struct Material 
{
    vec3 albedo;
    float roughness;
    float metallic;
    vec3 emission;
};

struct Light 
{
    vec3 direction;
    vec3 color;
    float intensity;
};

vec3 fresnelSchlick(float cosTheta, vec3 F0) 
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

float distributionGGX(vec3 N, vec3 H, float roughness) 
{
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;
    
    float num = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    
    return num / max(denom, 0.0000001);
}

float geometrySchlickGGX(float NdotV, float roughness) 
{
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;
    
    float num = NdotV;
    float denom = NdotV * (1.0 - k) + k;
    
    return num / denom;
}

float geometrySmith(vec3 N, vec3 V, vec3 L, float roughness) 
{
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    
    float ggx2 = geometrySchlickGGX(NdotV, roughness);
    float ggx1 = geometrySchlickGGX(NdotL, roughness);
    
    return ggx1 * ggx2;
}

vec3 calculatePBR(vec3 N, vec3 V, vec3 L, Material mat, Light light) 
{
    vec3 H = normalize(V + L);
    vec3 F0 = vec3(0.04); 
    F0 = mix(F0, mat.albedo, mat.metallic);
    
    float NDF = distributionGGX(N, H, mat.roughness);   
    float G   = geometrySmith(N, V, L, mat.roughness);      
    vec3 F    = fresnelSchlick(max(dot(H, V), 0.0), F0);       
    
    vec3 numerator    = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
    vec3 specular     = numerator / denominator;
    
    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - mat.metallic;
    
    float NdotL = max(dot(N, L), 0.0);        
    vec3 radiance = light.color * light.intensity;
    
    return (kD * mat.albedo / PI + specular) * radiance * NdotL;
}

// ============================================================================
// [10] 3D SCENE MAPPING & RAYMARCHING
// ============================================================================
vec2 mapScene(vec3 p) 
{
    vec3 c = vec3(18.0, 16.0, 22.0);
    vec3 id = floor((p + c * 0.5) / c);
    vec3 q = opRepetition(p, c);
    
    float h = hash13(id);
    
    // Sparsity check - leave open space in the middle for the 2D background
    if (h > 0.65) 
    { 
        // Procedural animation (Rotation)
        q.xy *= rot2D(u_time * 0.2 + h * TWO_PI);
        q.xz *= rot2D(u_time * 0.15 + h * PI);
        
        // Deformation
        q.y *= 0.35 + h * 0.45; 
        
        float baseScale = 1.0 + h * 1.8;
        float d = MAX_DIST;
        
        // Dynamic SDF selection for heterogeneous debris
        float structureHash = hash11(id.x * 2.3 + id.y * 5.1 + id.z * 7.7);
        
        if (structureHash < 0.3) 
        {
            d = sdOctahedron(q, baseScale);
        } 
        else if (structureHash < 0.6) 
        {
            d = sdHexPrism(q, vec2(baseScale * 0.6, baseScale * 1.3));
        } 
        else if (structureHash < 0.8) 
        {
            d = sdRoundBox(q, vec3(baseScale * 0.6), 0.2);
        } 
        else 
        {
            d = sdCylinder(q, vec3(0.0, 0.0, baseScale * 1.1));
            d = opSmoothSubtraction(sdSphere(q, baseScale * 0.8), d, 0.3);
        }
        
        // Fractal surface displacement for glassy cracks
        float disp = ridge3D_4(q * 5.0) * 0.18;
        d -= disp;
        
        return vec2(d, 1.0); 
    }
    
    return vec2(MAX_DIST, 0.0);
}

vec3 calculateNormal(vec3 p) 
{
    vec2 e = vec2(EPSILON, 0.0);
    return normalize(vec3(
        mapScene(p + e.xyy).x - mapScene(p - e.xyy).x,
        mapScene(p + e.yxy).x - mapScene(p - e.yxy).x,
        mapScene(p + e.yyx).x - mapScene(p - e.yyx).x
    ));
}

float calculateSoftShadow(vec3 ro, vec3 rd, float mint, float maxt, float k) 
{
    float res = 1.0;
    float t = mint;
    
    for (int i = 0; i < 32; i++) 
    {
        float h = mapScene(ro + rd * t).x;
        res = min(res, k * h / t);
        t += clamp(h, 0.02, 0.25);
        if (res < 0.005 || t > maxt) break;
    }
    return clamp(res, 0.0, 1.0);
}

float calculateAmbientOcclusion(vec3 pos, vec3 nor) 
{
    float occ = 0.0;
    float sca = 1.0;
    
    for (int i = 0; i < 5; i++) 
    {
        float h = 0.01 + 0.12 * float(i) / 4.0;
        float d = mapScene(pos + h * nor).x;
        occ += (h - d) * sca;
        sca *= 0.95;
    }
    return clamp(1.0 - 3.0 * occ, 0.0, 1.0);
}

// ============================================================================
// [11] 2D PLANETARY SYSTEM RENDERER
// ============================================================================
vec4 renderPlanetSystem(
    vec2 uv, 
    vec2 center, 
    float radius, 
    vec3 pColor, 
    vec3 iceColor, 
    vec2 lightDir, 
    vec2 offset
) {
    vec2 p = uv - center;
    float d = length(p);
    
    // Core anti-aliasing mask
    float mask = smoothstep(radius + 0.005, radius - 0.005, d);
    
    // Atmospheric optical bleeding (Halo)
    float haloIntensity = exp(-(d - radius) * 12.0);
    float haloLit = smoothstep(-radius, radius * 2.0, dot(p, lightDir));
    vec3 halo = iceColor * haloIntensity * haloLit * 0.8;
    
    if (mask <= 0.0) 
    {
        return vec4(halo, 0.0); 
    }
    
    // Pseudo 3D mapped sphere normal
    float z = sqrt(max(0.0, radius * radius - d * d));
    vec3 normal = normalize(vec3(p.x, p.y, z));
    vec3 l = normalize(vec3(lightDir.x, lightDir.y, 0.7));
    
    // Sphere-mapped UV coordinates for texturing
    vec2 sp = normal.xy * 2.8 + center * 12.0 + offset;
    
    // Complex Ice Topography
    vec3 v3 = voronoi3D(vec3(sp * 4.0, u_time * 0.015));
    float cracks = v3.x;
    float terrain = fbm2D_6(sp * 5.0);
    
    // Diffuse wrap lighting
    float diff = max(dot(normal, l), 0.0);
    float wrapDiff = max(0.0, (dot(normal, l) + 0.4) / 1.4);
    
    // Specular highlight
    vec3 view = vec3(0.0, 0.0, 1.0);
    vec3 halfV = normalize(l + view);
    float spec = pow(max(dot(normal, halfV), 0.0), 48.0) * terrain;
    
    // Material blending
    vec3 albedo = mix(pColor, iceColor, terrain + 0.15);
    albedo = mix(albedo, vec3(0.95, 0.98, 1.0), smoothstep(0.35, 0.75, 1.0 - cracks));
    
    // Inner Rim Light (Fresnel)
    float rim = pow(1.0 - max(dot(normal, view), 0.0), 3.5);
    
    vec3 finalCol = albedo * wrapDiff;
    finalCol += iceColor * rim * diff * 3.0;
    finalCol += vec3(1.0) * spec;
    
    return vec4(finalCol, mask);
}

// ============================================================================
// [12] COLOR GRADING & POST-PROCESSING
// ============================================================================
vec3 ACESFilm(vec3 x) 
{
    float a = 2.51;
    float b = 0.03;
    float c = 2.43;
    float d = 0.59;
    float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

vec3 applyVignette(vec3 color, vec2 fragUv, float intensity, float smoothness) 
{
    float dist = distance(fragUv, vec2(0.5));
    float vignette = smoothstep(intensity, intensity - smoothness, dist);
    return color * vignette;
}

vec3 applyChromaticAberration(vec3 color, vec2 fragUv, float strength) 
{
    vec2 offset = (fragUv - 0.5) * strength;
    // Since true CA requires sampling the whole buffer again, 
    // we simulate it locally by slightly shifting channels based on radial distance
    float r = color.r + 0.03 * length(offset);
    float b = color.b + 0.03 * length(-offset);
    return vec3(r, color.g, b);
}

// ============================================================================
// [13] MAIN RENDER PIPELINE
// ============================================================================
void main() 
{
    // Screen Space and Corrected UVs
    vec2 fragUv = gl_FragCoord.xy / u_resolution.xy;
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution.xy) / u_resolution.y;
    
    // UI Zoom Integration
    uv /= u_zoom;
    
    // Spiral coordinate manipulation tied to zoom factor
    float spiralAmount = (1.0 - u_zoom) * 1.8;
    uv *= rot2D(spiralAmount);
    
    // Continuous Parallax logic
    vec2 pan = vec2(u_pan.x, -u_pan.y);
    vec2 flow = u_flow_offset * 0.0001;
    
    vec3 outColor = vec3(0.0);

    // --------------------------------------------------------
    // PASS 1: Deep Cosmic Background & Volumetric Nebula
    // --------------------------------------------------------
    vec3 skyBase = mix(vec3(0.01, 0.015, 0.03), vec3(0.04, 0.07, 0.15), length(uv));
    
    vec3 warpPos = vec3(uv * 1.5 + pan * 0.0001 - flow * 0.4, u_time * 0.012);
    float nebulaDensity = domainWarp3D(warpPos, u_time * 0.005);
    
    vec3 nebColor1 = vec3(0.05, 0.25, 0.45);
    vec3 nebColor2 = vec3(0.2, 0.1, 0.35);
    vec3 nebMix = mix(nebColor1, nebColor2, fbm3D_5(warpPos * 2.5));
    
    skyBase += nebMix * smoothstep(0.25, 0.85, nebulaDensity) * 0.9;
    
    // Parallax Starfield (3 Layers)
    for (float i = 1.0; i <= 3.0; i += 1.0) 
    {
        vec2 sp = uv * (35.0 + i * 15.0) + pan * 0.0005 * i;
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i * 4.14);
        
        if (h > 0.94) 
        {
            float size = (h - 0.94) * 22.0;
            float glow = exp(-length(fp) * (35.0 - size));
            float twinkle = 0.5 + 0.5 * sin(u_time * 4.0 + h * 120.0);
            skyBase += mix(vec3(0.6, 0.8, 1.0), vec3(1.0, 0.9, 0.8), fract(h * 33.3)) * glow * twinkle;
        }
    }
    
    outColor = skyBase;

    // --------------------------------------------------------
    // PASS 2: 2D Projected Planetary System
    // --------------------------------------------------------
    vec2 globalLightDir2D = normalize(vec2(0.8, 0.65));
    
    // Distant Background Planet (Ice Moon)
    vec4 planet1 = renderPlanetSystem(
        uv, vec2(0.75, 0.35) - pan * 0.0002, 0.10, 
        vec3(0.05, 0.08, 0.12), vec3(0.4, 0.6, 0.8), globalLightDir2D, pan * 0.001
    );
    outColor = mix(outColor + planet1.rgb * (1.0 - planet1.a), planet1.rgb, planet1.a);
    
    // Midground Planet (Fractured Ice Giant)
    vec4 planet2 = renderPlanetSystem(
        uv, vec2(-0.65, -0.35) - pan * 0.0005, 0.32, 
        vec3(0.1, 0.15, 0.25), vec3(0.7, 0.9, 1.0), globalLightDir2D, pan * 0.002
    );
    outColor = mix(outColor + planet2.rgb * (1.0 - planet2.a), planet2.rgb, planet2.a);

    // --------------------------------------------------------
    // PASS 3: High-Velocity Volumetric Snow Drift Parallax
    // --------------------------------------------------------
    vec2 snowWind = vec2(-u_time * 0.6, u_time * 0.4) - flow * 4.0;
    
    for (float i = 1.0; i <= 5.0; i += 1.0) 
    {
        vec2 sp = uv * (22.0 / i) + pan * 0.002 * i + snowWind * (1.0 / i);
        sp.x += sin(sp.y * 2.5 + u_time) * 0.12; // Turbulent wind mapping
        
        vec2 id = floor(sp);
        vec2 fp = fract(sp) - 0.5;
        float h = hash12(id + i * 8.88);
        
        if (h > 0.72) 
        {
            float blur = 0.03 + i * 0.018;
            float mask = smoothstep(blur, 0.0, length(fp));
            float motionBlur = smoothstep(0.15, 0.0, abs(fp.x + fp.y * 1.5));
            outColor += vec3(0.85, 0.95, 1.0) * mask * motionBlur * (h * 0.6 + 0.4);
        }
    }

    // --------------------------------------------------------
    // PASS 4: 3D Raymarching Engine (Foreground Glacial Debris)
    // --------------------------------------------------------
    // Ray Setup
    vec3 ro = vec3(pan.x * 0.005, -pan.y * 0.005, -7.0 + u_time * 0.5);
    vec3 rd = normalize(vec3(uv, 1.0));
    
    // Sync 3D ray orientation with the 2D zoom spiral
    rd.xy *= rot2D(spiralAmount); 
    
    float t = 0.0;
    float hitMaterial = 0.0;
    
    // Marching Loop
    for (int i = 0; i < MAX_STEPS; i++) 
    {
        vec2 d = mapScene(ro + rd * t);
        if (d.x < SURF_DIST) 
        {
            hitMaterial = d.y;
            break;
        }
        t += d.x;
        if (t > MAX_DIST) break;
    }
    
    // Ray Intersection Geometry Shading
    if (t < MAX_DIST && hitMaterial == 1.0) 
    {
        vec3 p = ro + rd * t;
        vec3 n = calculateNormal(p);
        vec3 v = normalize(ro - p);
        
        // Define Light source
        Light mainLight;
        mainLight.direction = normalize(vec3(0.8, 0.7, -0.6));
        mainLight.color = vec3(0.9, 0.95, 1.0);
        mainLight.intensity = 2.8;
        
        // Define Material properties for Glassy Ice
        Material iceMat;
        iceMat.albedo = vec3(0.15, 0.35, 0.55);
        iceMat.roughness = 0.04;
        iceMat.metallic = 0.1;
        
        // Execute PBR BRDF
        vec3 pbrLighting = calculatePBR(n, v, mainLight.direction, iceMat, mainLight);
        
        // Calculate ambient occlusion and soft shadowing
        float ao = calculateAmbientOcclusion(p, n);
        float shadow = calculateSoftShadow(p, mainLight.direction, 0.05, 5.0, 14.0);
        
        // Fake Subsurface Scattering (Ice Transmission)
        float sssIntensity = pow(max(0.0, dot(v, -mainLight.direction)), 3.0) * 0.6;
        vec3 sssColor = vec3(0.05, 0.4, 0.75) * sssIntensity;
        
        // Internal Refraction Simulation (Distorting the background)
        vec3 refRay = refract(rd, n, 1.0 / 1.309); // IOR of Ice
        vec2 refUv = uv + refRay.xy * 0.25;
        
        // Resample fog at distorted coordinate
        float refFog = domainWarp3D(vec3(refUv * 2.0, u_time * 0.01), 0.0);
        vec3 refColor = mix(vec3(0.02, 0.05, 0.1), vec3(0.3, 0.6, 0.9), refFog) * 1.8;
        
        // Composite Final Material
        vec3 crystalColor = (pbrLighting * shadow) + sssColor + (refColor * 0.8);
        crystalColor *= ao;
        
        // Cinematic Depth of Field (Blur Falloff)
        float dofFocusPoint = 9.0;
        float dofRange = 18.0;
        float blurAmount = smoothstep(0.0, dofRange, abs(t - dofFocusPoint));
        
        // Blend raymarched crystals over the 2D background
        outColor = mix(crystalColor, outColor, blurAmount * 0.88);
    }

    // --------------------------------------------------------
    // PASS 5: Post-Processing & Tonemapping
    // --------------------------------------------------------
    
    // 1. Film Tonemapping (ACES curve)
    outColor = ACESFilm(outColor);
    
    // 2. Chromatic Aberration 
    outColor = applyChromaticAberration(outColor, fragUv, 0.8);
    
    // 3. Cinematic Vignette (Preserving absolute center for UI visibility)
    outColor = applyVignette(outColor, fragUv, 0.85, 0.5);
    
    // 4. Contrast & Lift Adjustment
    outColor = smoothstep(0.0, 1.05, outColor);
    outColor = mix(outColor, vec3(0.0), 0.015); // Slight black lift
    
    // Final GL Output
    gl_FragColor = vec4(clamp(outColor, 0.0, 1.0), 1.0) * alpha;
}