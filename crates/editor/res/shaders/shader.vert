#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;
layout(location = 3) in uvec4 inJoints;
layout(location = 4) in vec4 inWeights;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec3 fragNormal;

layout(push_constant) uniform PushConstants {
    mat4 mvp;
    mat4 model;
} pc;

layout(set = 0, binding = 0) readonly buffer Joints {
    mat4 joint_matrices[];
};

void main() {
vec4 w = inWeights / max(dot(inWeights, vec4(1.0)), 1e-6);
mat4 skin = w.x * joint_matrices[inJoints.x] + w.y * joint_matrices[inJoints.y]
          + w.z * joint_matrices[inJoints.z] + w.w * joint_matrices[inJoints.w];
    vec4 skinned = skin * vec4(inPosition, 1.0);
    gl_Position = pc.mvp * skinned;
    // Vulkan's NDC y-axis points down, but the camera matrix is written for
    // OpenGL (nalgebra, y-up). Flip so the image renders upright; the CPU-side
    // projection used for picking keeps its OpenGL convention.
    gl_Position.y = -gl_Position.y;

    vec3 skinned_normal = mat3(skin) * inNormal;
    fragNormal = normalize(mat3(pc.model) * skinned_normal);
    fragColor = vec3(1.0, 1.0, 1.0);
}


