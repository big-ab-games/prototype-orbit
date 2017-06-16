#version 330 core

in vec2 model;
in float alpha_base;

out vec4 out_color;
const float border_dist = 0.81;

void main() {
    float dist = pow(model.x, 2) + pow(model.y, 2);
    if (dist > 1.0) {
        discard;
    }
    vec3 color = vec3(1.0, 1.0, 1.0);
    if (dist > border_dist) {
        float fade_factor = (dist - border_dist) / (1.0 - border_dist);
        float alpha = mix(1.0, 0.0, fade_factor);
        out_color = vec4(color, alpha);
    }
    else {
        out_color = vec4(color, 1.0);
    }
}
