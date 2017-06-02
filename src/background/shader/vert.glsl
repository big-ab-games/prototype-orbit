#version 330 core

layout(std140) uniform;

uniform global_transform {
    mat4 view;
    mat4 proj;
};

in vec2 position;

void main() {
    gl_Position =  proj * view * vec4(position, 1.0, 1.0);
}
