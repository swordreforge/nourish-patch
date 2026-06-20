#version 450
// Composite quad: generated from gl_VertexIndex (triangle strip, 4 verts).
// Push constants carry dst rect in NDC and src rect in UV space.
layout(push_constant) uniform Push {
    vec4 dst;    // x, y, w, h in NDC
    vec4 src;    // u, v, w, h in UV
    vec4 color;  // rgba for solid; (1,1,1,alpha) for textured
} pc;

layout(location = 0) out vec2 v_uv;

void main() {
    vec2 corner = vec2(float(gl_VertexIndex & 1), float((gl_VertexIndex >> 1) & 1));
    vec2 pos = pc.dst.xy + corner * pc.dst.zw;
    v_uv = pc.src.xy + corner * pc.src.zw;
    gl_Position = vec4(pos, 0.0, 1.0);
}
