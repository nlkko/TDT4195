#version 450 core

layout(location=1) in vec4 in_colour;
layout(location=3) in vec3 in_normals;

out vec4 colour;

void main()
{
    vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));
    vec3 coluor_temp = vec3(in_colour[0], in_colour[1], in_colour[2]) * max(dot(in_normals, -lightDirection), 0);
    colour = vec4(coluor_temp[0], coluor_temp[1], coluor_temp[2], in_colour[3]);
}