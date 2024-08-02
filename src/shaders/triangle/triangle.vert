#version 450

layout(location = 0) in vec2 pos;
layout(location = 1) in vec3 color;

struct PerObject {
    mat4 mat;
    vec3 color_offset;
};

layout(constant_id = 0) const uint NUM_OBJECTS = 64;

layout(set = 1, binding = 0) uniform PerPass0 {
    mat4 view;
    mat4 proj;
} per_pass_0;

layout(set = 3, binding = 0) uniform Objects0 {
    PerObject[NUM_OBJECTS] objects;
} objects_0;

layout(location = 0) out vec3 o_color;

void main() {
    uint index = gl_InstanceIndex;

    mat4 worldview = per_pass_0.view * objects_0.objects[index].mat;
    //v_normal = transpose(inverse(mat3(worldview))) * normal;
    vec4 position = per_pass_0.proj * worldview * vec4(pos, 0.0, 1.0);
    gl_Position = position;
    
    o_color = color + objects_0.objects[index].color_offset;
}  