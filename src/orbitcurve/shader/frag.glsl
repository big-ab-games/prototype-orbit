#version 330 core

layout(std140) uniform;

struct OrbitCurveBezier {
    vec2 p1;
    vec2 p2;
    vec2 p3;
    float opacity;
};

uniform beziers {
    OrbitCurveBezier u_beziers[1024]; // TODO hardcode max instances, can do better?
};

const float thickness = 0.08;

in vec2 model_pos;
flat in uint bezier_idx;

out vec4 out_color;

vec2 linear_bezier(float t, vec2 p1, vec2 p2) {
    return (1 - t) * p1 + t * p2;
}

vec2 quad_bezier(float t, vec2 p1, vec2 p2, vec2 p3) {
    return pow(1-t, 2) * p1 +
        2 * (1-t) * t * p2 +
        t * t * p3;
}

vec2 cubic_bezier(float t, vec2 p1, vec2 p2, vec2 p3, vec2 p4) {
    return pow(1-t, 3) * p1 +
        3 * pow(1-t, 2) * t * p2 +
        3 * (1-t) * pow(t, 2) * p3 +
        pow(t, 3) * p4;
}

/// :t float in [0,1]
float distance_from_curve_at(float t) {
    OrbitCurveBezier bezier = u_beziers[bezier_idx];

    // vec2 pt = cubic_bezier(t, bezier.p1, bezier.p2, bezier.p2, bezier.p3);
    // vec2 pt = quad_bezier(t, bezier.p1, bezier.p2, bezier.p3);
    vec2 pt = linear_bezier(t, bezier.p1, bezier.p3);

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
    float distance_from_curve = distance_from_curve();
    float max_dist = thickness / 2;
    if (distance_from_curve <= max_dist) {
        out_color = vec4(1.0, 1.0, 1.0, 1 - u_beziers[bezier_idx].opacity);
        // blend anti-alias
        out_color.a *= mix(1.0, 0.0, distance_from_curve / max_dist);
    }
    else discard;
}
