#version 450
layout(push_constant) uniform Push {
    vec4 dst;
    vec4 src;
    vec4 color; // (1,1,1,alpha)
} pc;

layout(set = 0, binding = 0) uniform sampler2D u_tex;
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 o_color;

void main() {
    o_color = texture(u_tex, v_uv) * pc.color;
}
