#version 330 core

in vec2 model;
in float alpha_base;

out vec4 out_color;

void main() {
    float dist = sqrt(model.x * model.x + model.y * model.y);
    if (dist > 1.0) {
        discard;
    }
    out_color = vec4(1.0, 1.0, 1.0, 1.0);
}
