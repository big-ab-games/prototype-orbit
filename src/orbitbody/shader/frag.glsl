#version 330 core

in vec2 model;
in float alpha_base;

out vec4 out_color;
const float border_dist = 0.9;

void main() {
    float dist = sqrt(model.x * model.x + model.y * model.y);
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
        out_color = vec4(1.0, 1.0, 1.0, 1.0);
    }
}
