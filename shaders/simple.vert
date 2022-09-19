#version 450 core

in vec3 position;
layout(location=1) in vec4 in_olour;
layout(location=1) out vec4 out_colour;

float a = 1.0f;
float b = 0.0f;
float c = 0.0f;
float d = 0.0f;
float e = 1.0f;
float f = 0.0f;

mat4 matrix = mat4(
    a, b, 0, c,
    d, e, 0, f,
    0, 0, 1, 0,
    0, 0, 0, 1
);

void main()
{
    gl_Position = matrix * vec4(position, 1.0f);
    out_colour = in_olour;
}