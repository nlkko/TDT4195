#version 450 core

layout(location=1) in vec4 in_color;
layout(location=3) in vec3 in_normals;

out vec4 color;

void main()
{
    color = vec4(in_normals[0], in_normals[1], in_normals[2], in_color[3]);
}