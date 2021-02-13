#version 450

layout (location = 0) in vec2 fragUV;

layout (location = 0) out vec4 outFragColor;

layout (set = 0, binding = 0) uniform texture2D displayTexture;
layout (set = 0, binding = 1) uniform sampler samp;

void main()
{
    vec3 color = texture(sampler2D(displayTexture, samp), fragUV).rgb;
    outFragColor = vec4(color, 1.0);
}
