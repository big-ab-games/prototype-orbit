#version 330 core

layout(std140) uniform;

struct OrbitCurveBezier {
    vec2 p1;
    vec2 p2;
    float opacity;
    float thickness;
};

uniform beziers {
    OrbitCurveBezier u_beziers[1024]; // TODO hardcode max instances, can do better?
};

in vec2 model_pos;
flat in uint bezier_idx;

out vec4 out_color;

vec2 linear_bezier(float t, vec2 p1, vec2 p2) {
    return (1 - t) * p1 + t * p2;
}

/// :t float in [0,1]
float distance_from_curve_at(float t) {
    OrbitCurveBezier bezier = u_beziers[bezier_idx];

    vec2 pt = linear_bezier(t, bezier.p1, bezier.p2);

    return distance(pt, model_pos);
}

float distance_from_curve() {
    float from = 0.0;
    float end = 1.0;
    float dist_from = distance_from_curve_at(from);
    float dist_end = distance_from_curve_at(end);

    while(abs(from - end) > 0.00001) {
        if (dist_end < dist_from) {
            from = (from + from + end) / 3.0;
        }
        else {
            end = (from + end + end) / 3.0;
        }
        dist_from = distance_from_curve_at(from);
        dist_end = distance_from_curve_at(end);
    }

    return min(dist_end, dist_from);
}

void main() {
    OrbitCurveBezier bezier = u_beziers[bezier_idx];

    float distance_from_curve = distance_from_curve();
    float max_dist = bezier.thickness / 2;
    if (distance_from_curve <= max_dist) {
        out_color = vec4(1.0, 1.0, 1.0, bezier.opacity);
        // blend anti-alias
        out_color.a *= mix(1.0, 0.0, distance_from_curve / max_dist);
    }
    else discard;
}
