#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec3 color;

struct PerObject {
    vec2 offset;
    vec2 scale;
    vec3 color_offset;
};

layout(set = 0, binding = 0) uniform B0 {
    float aspect_ratio;
    float viewport_scale;
    vec2 viewport_offset;
    float time;
    float _0;
    vec2 _1;
    PerObject[64] objects;
} ubo;

layout(location = 0) out vec3 o_color;

void main() {
    uint index = gl_InstanceIndex;

    vec2 tpos = pos * ubo.objects[index].scale;
    tpos = tpos + ubo.objects[index].offset;
    tpos.x *= ubo.aspect_ratio; 
    tpos *= ubo.viewport_scale;
    tpos += ubo.viewport_offset * ubo.aspect_ratio;
    gl_Position = vec4(tpos, 0.0, 1.0);

    o_color = color + ubo.objects[index].color_offset;
}  