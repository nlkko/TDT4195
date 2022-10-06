#version 450 core

layout(location=0) in vec3 position;
layout(location=1) in vec4 in_olour;
layout(location=1) out vec4 out_colour;
uniform layout(location=2) mat4 transformation_matrix;
layout(location=3) in vec3 in_normals;
layout(location=3) out vec3 out_normals;

void main()
{
    gl_Position = transformation_matrix * vec4(position, 1.0f);
    out_colour = in_olour;
    out_normals = in_normals;
}