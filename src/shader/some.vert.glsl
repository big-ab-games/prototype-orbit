#version 330 core

in vec2 position;
in uint local_idx;

uniform global_transform {
    mat4 view;
    mat4 proj;
};

struct OrbitBodyTransform {
    mat4 transform;
};
uniform local_transform {
    OrbitBodyTransform locals[3];
};

void main() {
    mat4 local = locals[local_idx].transform;
    gl_Position = proj * view * local * vec4(position, 0.0, 1.0);
}
