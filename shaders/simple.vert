#version 450 core

in vec3 position;
layout(location=1) in vec4 in_olour;
layout(location=1) out vec4 out_colour;

void main()
{
    gl_Position = vec4(position, 1.0f);
    out_colour = in_olour;
}