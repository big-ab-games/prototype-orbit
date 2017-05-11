#version 330 core

#define PI 3.1415926535897932384626433832795

in vec2 position;
in vec3 color;
in vec2 tex_coords;
out vec3 f_color;
out vec2 f_tex_coords;

uniform u_vals {
    float u_ticks;
};

float norm_sin(float minus_1_to_1) {
    return minus_1_to_1 / 2.0 + 0.5;
}

void main() {
    float pi_seconds = u_ticks * PI / 3000.0;
    mat2 rotate = mat2(
        cos(pi_seconds), -sin(pi_seconds),
        sin(pi_seconds), cos(pi_seconds)
    );
    float color_mod = norm_sin(cos(pi_seconds)) / 2 + 0.5;
    f_color = color;// vec3(rotate * color.rg, (rotate * color.gb).y);
    f_tex_coords = tex_coords;
    gl_Position = vec4(/*rotate **/ position, 0.0, 1.0);
}
