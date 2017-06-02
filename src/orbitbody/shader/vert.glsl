#version 330 core

layout(std140) uniform;

struct OrbitBodyTransform {
    mat4 transform;
};

uniform global_transform {
    mat4 view;
    mat4 proj;
};

uniform local_transform {
    OrbitBodyTransform locals[1];
};

in vec2 position;
in uint local_idx;

out vec2 model;

void main() {
    mat4 local = locals[local_idx].transform;
    model = position;
    gl_Position = proj * view * local * vec4(position, 0.5, 1.0);
}
