#version 330 core

in vec3 f_color;
in vec2 f_tex_coords;
out vec4 some_target;

uniform u_vals {
    float u_ticks;
};
uniform sampler2D t_happy;
uniform sampler2D t_sad;

float norm_sin(float minus_1_to_1) {
    return minus_1_to_1 / 2.0 + 0.5;
}

void main() {
    vec2 coords = f_tex_coords;

    float waterline = mix(0.63, 0.67, norm_sin(sin(u_ticks / 1000.)));
    bool water = coords.y > waterline;

    if (water) {
        // still
        // coords = vec2(coords.x, waterline * 2.0 - coords.y);

        // sin waves
        coords = vec2(coords.x + sin(coords.y * 40.0 + u_ticks / 300.0) / 30.0, waterline * 2.0 - coords.y);
    }

    vec4 happy = texture(t_happy, coords);
    vec4 sad = texture(t_sad, coords);
    some_target = mix(sad, happy, min(norm_sin(sin(u_ticks / 1000.0)) * 3.0, 1.0));
    if (water) // color the water a bit
        some_target = some_target * vec4(0.6, 0.7, 1.0, 1.0);

    some_target = some_target * vec4(f_color, 1.0);
}
