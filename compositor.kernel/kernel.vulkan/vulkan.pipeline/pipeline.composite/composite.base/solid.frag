#version 450
layout(push_constant) uniform Push {
    vec4 dst;
    vec4 src;
    vec4 color;
} pc;

layout(location = 0) out vec4 o_color;

void main() {
    o_color = pc.color;
}
