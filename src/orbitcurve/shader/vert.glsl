#version 330 core

layout(std140) uniform;

uniform global_transform {
    mat4 view;
    mat4 proj;
};

in vec2 position;
in uint local_idx;

out vec2 model_pos;
flat out uint bezier_idx;

void main() {
    bezier_idx = local_idx;
    model_pos = position;
    gl_Position = proj * view * vec4(position, 0.75, 1.0);
}
