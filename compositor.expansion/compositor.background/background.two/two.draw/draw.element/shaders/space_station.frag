precision highp float;

uniform float u_time;
uniform vec2  u_pan;
uniform vec2  u_flow_offset;
uniform float u_zoom;
uniform vec2  u_resolution;
uniform float alpha;

// ============================================================================
// Hash & noise
// ============================================================================
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

// ============================================================================
// SDF primitives
// ============================================================================
float sdBox(vec2 p, vec2 b){
    vec2 d=abs(p)-b;
    return length(max(d,0.))+min(max(d.x,d.y),0.);
}
float sdCircle(vec2 p, float r){ return length(p)-r; }
float sdRing(vec2 p, float r, float w){ return abs(length(p)-r) - w; }
// Rounded box for softer edges on big concrete structures
float sdRoundBox(vec2 p, vec2 b, float r){
    vec2 d = abs(p) - b + r;
    return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - r;
}
// Rotate point by angle
vec2 rot(vec2 p, float a){
    float c = cos(a), s = sin(a);
    return vec2(c*p.x - s*p.y, s*p.x + c*p.y);
}

// ============================================================================
// Composite shapes
// ============================================================================
// Lightning tower (lattice steel tower) as seen from above:
// = small square base + 4 diagonal struts radiating out as thin lines.
float lightning_tower(vec2 p) {
    float core = sdBox(p, vec2(0.008, 0.008));
    // Four diagonal struts (legs of the tower)
    float legs = 1e9;
    for (int i = 0; i < 4; i++) {
        float ang = float(i) * 1.5707963 + 0.7853982;  // 45° offsets
        vec2 dir = vec2(cos(ang), sin(ang));
        // Rotate p so the strut runs along x
        vec2 rp = rot(p, -ang);
        float strut = sdBox(rp - vec2(0.012, 0.0), vec2(0.012, 0.0015));
        legs = min(legs, strut);
    }
    return min(core, legs);
}

// Water tower: circle on top, 4 leg-dots below (top-down)
void water_tower(vec2 b, vec2 pos, inout vec3 col) {
    vec2 p = b - pos;
    // Tank (white, top-down circle)
    float tank = sdCircle(p, 0.028);
    col = mix(col, vec3(0.92, 0.92, 0.94), smoothstep(0.003, 0.0, tank));
    // Inner ring detail
    col = mix(col, vec3(0.70, 0.70, 0.72),
              smoothstep(0.003, 0.0, sdRing(p, 0.020, 0.002)));
    // Logo dot in center (faux dark text spot)
    col = mix(col, vec3(0.30, 0.30, 0.35),
              smoothstep(0.002, 0.0, sdCircle(p, 0.006)));
    // 4 leg shadows visible from above (small dots around tank)
    for (int i = 0; i < 4; i++) {
        float ang = float(i)*1.5707963 + 0.7853982;
        vec2 lp = p - vec2(cos(ang), sin(ang))*0.018;
        col = mix(col, vec3(0.40, 0.40, 0.42),
                  smoothstep(0.002, 0.0, sdCircle(lp, 0.003)));
    }
}

// Service road: long thin strip with stripe markings
void service_road(vec2 b, vec2 a_pt, vec2 b_pt, float width, inout vec3 col) {
    vec2 dir = normalize(b_pt - a_pt);
    vec2 nrm = vec2(-dir.y, dir.x);
    float len = length(b_pt - a_pt) * 0.5;
    vec2 center = (a_pt + b_pt) * 0.5;
    vec2 rp = b - center;
    rp = vec2(dot(rp, dir), dot(rp, nrm));

    float road = sdBox(rp, vec2(len, width));
    col = mix(col, vec3(0.42, 0.40, 0.38), smoothstep(0.003, 0.0, road));
    // Edge stripes
    col = mix(col, vec3(0.55, 0.52, 0.48),
              smoothstep(0.002, 0.0, abs(rp.y) - width + 0.003));
    // Dashed centerline
    float dash = step(0.5, fract(rp.x * 8.0));
    col = mix(col, mix(col, vec3(0.85, 0.80, 0.30), dash * 0.6),
              smoothstep(0.0015, 0.0, abs(rp.y)) *
              smoothstep(0.003, 0.0, road));
}

// Transporter-erector: rectangular base with cross-bracing visible from above
void transporter_erector(vec2 p, inout vec3 col) {
    // Main rectangular base
    float base = sdRoundBox(p, vec2(0.05, 0.10), 0.008);
    col = mix(col, vec3(0.45, 0.45, 0.48), smoothstep(0.003, 0.0, base));

    // Cross-bracing — two diagonal lines visible from above
    vec2 rp1 = rot(p, 0.7853982);    // 45°
    vec2 rp2 = rot(p, -0.7853982);   // -45°
    float diag1 = sdBox(rp1, vec2(0.001, 0.10));
    float diag2 = sdBox(rp2, vec2(0.001, 0.10));
    float diags = min(diag1, diag2);
    // Only draw diagonals INSIDE the base
    float baseFill = smoothstep(0.0, -0.005, base);
    col = mix(col, vec3(0.30, 0.30, 0.32),
              smoothstep(0.002, 0.0, diags) * baseFill);

    // Top rail (where rocket sits, slightly raised in color)
    col = mix(col, vec3(0.55, 0.55, 0.58),
              smoothstep(0.003, 0.0, sdBox(p, vec2(0.025, 0.10))));
}

// ============================================================================
// Main
// ============================================================================
void main() {
    vec2 screen_uv = (gl_FragCoord.xy - 0.5*u_resolution) / u_resolution.y;
    vec2 uv = screen_uv / u_zoom;

    // World coords — pan moves us across the surface
    vec2 world = uv + u_pan * 0.0008;

    // ====================================================================
    // GROUND: split into ocean (left), sand/dunes (right), tarmac (center)
    // ====================================================================
    // Distance from the central facility (in world coords) controls what
    // surface type appears.

    // Base color: tarmac/concrete
    vec3 col = vec3(0.42, 0.40, 0.36);

    // Add gravel/concrete texture
    float concreteNoise = fbm(world * 12.0) * 0.4 + fbm(world * 30.0) * 0.15;
    col *= 0.85 + concreteNoise * 0.4;

    // Sand dunes on the right side (world.x > 0.4)
    float sandMask = smoothstep(0.35, 0.55, world.x + fbm(world * 3.0) * 0.15);
    vec3 sandCol = vec3(0.55, 0.45, 0.32);
    float dunes = fbm(world * 6.0);
    sandCol *= 0.8 + dunes * 0.4;
    // Sparse scrub vegetation (tiny darker dots)
    float scrub = step(0.85, hash(floor(world * 80.0))) * 0.6;
    sandCol = mix(sandCol, vec3(0.25, 0.30, 0.18), scrub);
    col = mix(col, sandCol, sandMask);

    // Ocean on the left side (world.x < -0.4)
    float oceanMask = smoothstep(-0.35, -0.55, world.x - fbm(world * 2.0) * 0.15);
    // Animated water — small ripples drift always
    vec2 waveUv = world * 8.0 + vec2(u_time * 0.3, u_time * 0.15);
    float waves = fbm(waveUv) * 0.5 + fbm(waveUv * 2.5) * 0.3;
    vec3 oceanDeep    = vec3(0.10, 0.20, 0.32);
    vec3 oceanShallow = vec3(0.20, 0.40, 0.48);
    vec3 oceanCol = mix(oceanDeep, oceanShallow, waves);
    // Foam at the shoreline
    float shoreline = smoothstep(-0.36, -0.40, world.x);
    oceanCol = mix(oceanCol, vec3(0.85, 0.90, 0.92),
                   shoreline * (1.0 - shoreline) * 4.0 * step(0.6, fbm(world*15.0 + u_time*0.5)));
    col = mix(col, oceanCol, oceanMask);

    // Soft shoreline transition (wet sand)
    float wetSand = smoothstep(0.0, 0.05, abs(world.x + 0.40)) *
                    (1.0 - smoothstep(0.05, 0.10, abs(world.x + 0.40)));
    col = mix(col, vec3(0.35, 0.28, 0.20), wetSand * 0.5);

    // ====================================================================
    // FACILITY LAYOUT (centered around world.xy = 0)
    // ====================================================================
    vec2 b = world;

    // --- Main perimeter road (the asphalt around everything) ---
    float perimeter = sdRoundBox(b, vec2(0.32, 0.28), 0.04);
    col = mix(col, vec3(0.38, 0.36, 0.34),
              smoothstep(0.005, 0.0, perimeter) *
              smoothstep(-0.02, 0.0, perimeter));  // outline only

    // Inner concrete plaza
    float plaza = sdRoundBox(b, vec2(0.30, 0.26), 0.04);
    col = mix(col, vec3(0.50, 0.48, 0.44),
              smoothstep(0.005, 0.0, plaza) * 0.85);

    // --- Service roads radiating from the pad ---
    service_road(b, vec2(0.0, 0.0), vec2(0.0, -0.30), 0.014, col);  // south access
    service_road(b, vec2(0.0, 0.0), vec2(-0.25, 0.05), 0.010, col); // west road
    service_road(b, vec2(0.0, 0.0), vec2(0.25, 0.05), 0.010, col);  // east road

    // --- Hangar / horizontal integration facility (south of pad) ---
    {
        vec2 hp = b - vec2(0.0, -0.36);
        float hangar = sdRoundBox(hp, vec2(0.10, 0.04), 0.005);
        col = mix(col, vec3(0.62, 0.60, 0.58), smoothstep(0.004, 0.0, hangar));
        // Roof seams (parallel lines)
        for (int i = 0; i < 5; i++) {
            float xo = -0.08 + float(i) * 0.04;
            col = mix(col, vec3(0.45, 0.43, 0.41),
                      smoothstep(0.002, 0.0, sdBox(hp - vec2(xo, 0.0), vec2(0.001, 0.04))));
        }
        // Door at the north end (facing pad)
        col = mix(col, vec3(0.30, 0.28, 0.30),
                  smoothstep(0.003, 0.0, sdBox(hp - vec2(0.0, 0.04), vec2(0.04, 0.005))));
    }

    // --- Storage / propellant farm (west) ---
    for (int i = 0; i < 3; i++) {
        for (int j = 0; j < 2; j++) {
            vec2 tp = b - vec2(-0.22 - float(i)*0.04, -0.10 + float(j)*0.06);
            float tank = sdCircle(tp, 0.018);
            col = mix(col, vec3(0.85, 0.83, 0.80), smoothstep(0.003, 0.0, tank));
            // Highlight ring (catches "sun" from upper-left)
            col = mix(col, vec3(0.95, 0.92, 0.88),
                      smoothstep(0.002, 0.0, sdCircle(tp - vec2(-0.004, 0.004), 0.014)));
            // Cap
            col = mix(col, vec3(0.55, 0.52, 0.50),
                      smoothstep(0.002, 0.0, sdCircle(tp, 0.008)));
        }
    }

    // --- Lightning towers (4 around the pad — characteristic of launch sites) ---
    vec2 ltPos[4];
    ltPos[0] = vec2(-0.10, 0.10);
    ltPos[1] = vec2( 0.10, 0.10);
    ltPos[2] = vec2(-0.10,-0.10);
    ltPos[3] = vec2( 0.10,-0.10);
    for (int i = 0; i < 4; i++) {
        float lt = lightning_tower(b - ltPos[i]);
        col = mix(col, vec3(0.30, 0.28, 0.30), smoothstep(0.002, 0.0, lt));
        // Tiny blinking aviation light at center
        float blink = 0.5 + 0.5*sin(u_time*2.0 + float(i)*1.5);
        col += vec3(1.0, 0.3, 0.2) *
               smoothstep(0.005, 0.0, length(b - ltPos[i])) * blink;
    }

    // --- Water tower (NE corner) ---
    water_tower(b, vec2(0.24, 0.16), col);

    // --- Small support buildings (north and east) ---
    for (int i = 0; i < 3; i++) {
        vec2 bp = b - vec2(-0.18 + float(i)*0.12, 0.20);
        float bld = sdRoundBox(bp, vec2(0.025, 0.018), 0.003);
        col = mix(col, vec3(0.50, 0.42, 0.36), smoothstep(0.003, 0.0, bld));
        // Roof line
        col = mix(col, vec3(0.30, 0.25, 0.22),
                  smoothstep(0.002, 0.0, sdBox(bp, vec2(0.025, 0.001))));
        // Door
        col = mix(col, vec3(0.20, 0.18, 0.18),
                  smoothstep(0.002, 0.0, sdBox(bp - vec2(0.0, -0.015), vec2(0.004, 0.003))));
    }

    // --- The launch pad itself ---
    vec2 padPos = b;
    float padOuter = length(padPos);

    // Pad concrete (round)
    col = mix(col, vec3(0.55, 0.52, 0.48), smoothstep(0.18, 0.16, padOuter));
    // Flame trench (dark cross beneath pad — the flame deflector)
    col = mix(col, vec3(0.12, 0.10, 0.08),
              smoothstep(0.003, 0.0, sdBox(padPos, vec2(0.14, 0.012))));
    col = mix(col, vec3(0.12, 0.10, 0.08),
              smoothstep(0.003, 0.0, sdBox(padPos, vec2(0.012, 0.14))));
    // Yellow safety markings (concentric)
    col = mix(col, vec3(0.85, 0.72, 0.20),
              smoothstep(0.003, 0.0, sdRing(padPos, 0.155, 0.003)));
    col = mix(col, vec3(0.85, 0.72, 0.20),
              smoothstep(0.003, 0.0, sdRing(padPos, 0.13, 0.002)));
    // Hold-down clamps (4 small rectangles around the pad center)
    for (int i = 0; i < 4; i++) {
        float ang = float(i) * 1.5707963;
        vec2 cp = vec2(cos(ang), sin(ang)) * 0.04;
        float clamp_d = sdBox(padPos - cp, vec2(0.008, 0.005));
        col = mix(col, vec3(0.30, 0.28, 0.28), smoothstep(0.002, 0.0, clamp_d));
    }

    // --- Transporter-erector & rocket ---
    transporter_erector(padPos, col);

    // ROCKET: viewed from above = circle (booster) with strap-on side boosters
    float coreBooster = sdCircle(padPos, 0.025);
    col = mix(col, vec3(0.95, 0.93, 0.92), smoothstep(0.003, 0.0, coreBooster));
    // Light highlight on core
    col = mix(col, vec3(1.0, 0.98, 0.96),
              smoothstep(0.002, 0.0, sdCircle(padPos - vec2(-0.005, 0.005), 0.018)));

    // Two side boosters (left and right, smaller circles — like Falcon Heavy)
    vec2 sb1 = padPos - vec2(-0.035, 0.0);
    vec2 sb2 = padPos - vec2( 0.035, 0.0);
    float sBooster1 = sdCircle(sb1, 0.018);
    float sBooster2 = sdCircle(sb2, 0.018);
    col = mix(col, vec3(0.90, 0.88, 0.88), smoothstep(0.003, 0.0, sBooster1));
    col = mix(col, vec3(0.90, 0.88, 0.88), smoothstep(0.003, 0.0, sBooster2));
    // Highlights
    col = mix(col, vec3(0.98, 0.95, 0.94),
              smoothstep(0.002, 0.0, sdCircle(sb1 - vec2(-0.003, 0.003), 0.012)));
    col = mix(col, vec3(0.98, 0.95, 0.94),
              smoothstep(0.002, 0.0, sdCircle(sb2 - vec2(-0.003, 0.003), 0.012)));

    // Engine bells (dark circles in core and side boosters)
    col = mix(col, vec3(0.10, 0.08, 0.08),
              smoothstep(0.002, 0.0, sdCircle(padPos, 0.012)));
    col = mix(col, vec3(0.10, 0.08, 0.08),
              smoothstep(0.002, 0.0, sdCircle(sb1, 0.008)));
    col = mix(col, vec3(0.10, 0.08, 0.08),
              smoothstep(0.002, 0.0, sdCircle(sb2, 0.008)));

    // Engine pre-burn glow (idling — flickers)
    float flicker = 0.85 + 0.15*sin(u_time*30.0) + 0.1*hash(vec2(floor(u_time*40.0), 1.0));
    col += vec3(1.0, 0.55, 0.25) *
           smoothstep(0.020, 0.0, length(padPos)) * flicker * 0.35;
    col += vec3(1.0, 0.55, 0.25) *
           smoothstep(0.014, 0.0, length(sb1)) * flicker * 0.3;
    col += vec3(1.0, 0.55, 0.25) *
           smoothstep(0.014, 0.0, length(sb2)) * flicker * 0.3;

    // ====================================================================
    // VEHICLES on the access road (animated — always moving)
    // ====================================================================
    // A truck driving in/out — loops along the south road
    {
        float t = mod(u_time * 0.08, 1.0);
        vec2 truckPos = mix(vec2(0.0, -0.45), vec2(0.0, -0.20), t);
        vec2 tp = b - truckPos;
        col = mix(col, vec3(0.85, 0.20, 0.18),
                  smoothstep(0.003, 0.0, sdRoundBox(tp, vec2(0.008, 0.012), 0.002)));
    }

    // ====================================================================
    // ATMOSPHERIC EFFECTS (top-down, animated)
    // ====================================================================

    // Drifting fog/haze across the whole scene — driven by time + flow
    vec2 fogFlow = -u_flow_offset * 0.0006 + vec2(u_time*0.04, u_time*0.025);
    float fog1 = fbm(world * 1.2 + fogFlow);
    float fog2 = fbm(world * 3.0 + fogFlow*1.5);
    float fog  = fog1 * 0.6 + fog2 * 0.4;
    float vig  = smoothstep(0.5, 1.2, length(screen_uv));
    float fogAmount = fog * 0.40 + vig * 0.30;
    fogAmount = clamp(fogAmount, 0.0, 0.75);
    vec3 fogCol = vec3(0.75, 0.78, 0.85);  // realistic atmospheric haze (bluish-grey)
    col = mix(col, fogCol, fogAmount);

    // Steam plumes from propellant tanks (expanding rings, always animated)
    for (int i = 0; i < 6; i++) {
        int ix = i / 2;
        int iy = i - ix * 2;
        vec2 tp = world - vec2(-0.22 - float(ix)*0.04, -0.10 + float(iy)*0.06);
        float phase = mod(u_time*0.4 + float(i)*0.7, 1.0);
        float r = phase * 0.10;
        float ringFade = (1.0 - phase) * (1.0 - phase);
        float steam = smoothstep(r, r*0.5, length(tp)) * ringFade;
        col = mix(col, vec3(0.92, 0.93, 0.95), steam * 0.25);
    }

    // Rocket exhaust pre-burn (expanding ring under engine, always animated)
    {
        float phase = mod(u_time * 1.5, 1.0);
        float r = phase * 0.08;
        float ringFade = (1.0 - phase);
        float ring = smoothstep(r + 0.006, r - 0.006, length(padPos)) *
                     smoothstep(r - 0.020, r, length(padPos)) * ringFade;
        col += vec3(1.0, 0.75, 0.35) * ring * 0.5;
    }

    // Big ground steam from flame trench (the launch-style billow, but static)
    {
        // Steam coming out of the cross-shaped flame trench
        float trench_h = max(
            smoothstep(0.012, 0.005, abs(padPos.y)) * smoothstep(0.14, 0.12, abs(padPos.x)),
            smoothstep(0.012, 0.005, abs(padPos.x)) * smoothstep(0.14, 0.12, abs(padPos.y))
        );
        float plume_time = u_time * 0.5;
        float plume_n = fbm(padPos * 8.0 + vec2(plume_time, 0.0)) *
                        fbm(padPos * 20.0 + vec2(0.0, plume_time*1.5));
        col = mix(col, vec3(0.95, 0.93, 0.92),
                  trench_h * plume_n * 0.6);
    }

    gl_FragColor = vec4(col, 1.0) * alpha * 0.85;
}