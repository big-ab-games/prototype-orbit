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
    // wiggle waterline in y-axis
    waterline = waterline + sin(coords.x * 50. + u_ticks / 80.0) / 400.0;
    bool water = coords.y > waterline;

    if (water) {
        // sin waves wiggle water in x-axis
        coords = vec2(coords.x + sin((coords.y - waterline) * 30.0 + u_ticks / 300.0) / 50.0,
                      waterline * 2.0 - coords.y);
    }

    vec4 happy = texture(t_happy, coords);
    vec4 sad = texture(t_sad, coords);
    some_target = mix(sad, happy, min(norm_sin(sin(u_ticks / 1000.0)) * 3.0, 1.0));
    if (water) // color the water a bit
        some_target = some_target * vec4(0.6, 0.90, 1.0, 1.0);

    // make the water a little transparent
    happy = texture(t_happy, f_tex_coords);
    sad = texture(t_sad, f_tex_coords);
    some_target = mix(some_target,
                      mix(sad, happy, min(norm_sin(sin(u_ticks / 1000.0)) * 3.0, 1.0)),
                      0.1);
}
