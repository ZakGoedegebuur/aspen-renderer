#version 450

layout(location = 0) in vec3 o_color;

layout(location = 0) out vec4 f_color;

//layout(set = 1, binding = 0) uniform sampler spl;
//layout(set = 1, binding = 1) uniform texture2D textures[];

void main() {
    //f_color = texture(sampler2D(textures[0], spl), vec2(0.2, 0.7));
    f_color = vec4(o_color, 1.0);
    //f_color += vec4(o_color, 1.0);
}