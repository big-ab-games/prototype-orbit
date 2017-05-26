#version 330 core

uniform global_transform {
    mat4 view;
    mat4 proj;
};

struct OrbitBodyTransform {
    mat4 transform;
};

uniform local_transform {
    OrbitBodyTransform locals[1024]; // TODO hardcode max instances, can do better?
};

in vec2 position;
in uint local_idx;

out vec2 model;

void main() {
    mat4 local = locals[local_idx].transform;
    model = position;
    gl_Position = proj * view * local * vec4(position, 0.5, 1.0);
}
