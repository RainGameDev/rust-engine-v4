#version 450

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec3 fragNormal;

layout(location = 0) out vec4 outColor;

layout(set = 1, binding = 0) uniform sampler2D texSampler;

void main() {
    vec4 texColor = texture(texSampler, fragUV);
    vec3 normal = normalize(fragNormal);
    vec3 light_dir = normalize(vec3(0.5, 0.8, 0.4));
    float diffuse = max(dot(normal, light_dir), 0.0);
    vec3 shaded = texColor.rgb * (0.25 + 0.75 * diffuse);
    outColor = vec4(shaded, texColor.a);
}
