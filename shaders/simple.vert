#version 450 core

layout(location=0) in vec3 position;

layout(location=1) in vec4 in_colour;
layout(location=1) out vec4 out_colour;

layout(location=2) uniform mat4 transformation_matrix;

layout(location=3) in vec3 in_normals;
layout(location=3) out vec3 out_normals;

layout(location=4) uniform mat4 model_matrix;

void main()
{
    gl_Position = transformation_matrix * vec4(position, 1.0f);
    out_colour = in_colour;
    out_normals = normalize(vec3(model_matrix * vec4(in_normals, 0.0)));
}