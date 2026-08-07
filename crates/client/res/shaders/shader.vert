#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;
layout(location = 3) in uvec4 inJoints;
layout(location = 4) in vec4 inWeights;

layout(location = 0) out vec3 fragColor;

layout(push_constant) uniform PushConstants {
    mat4 mvp;
} pc;

layout(set = 0, binding = 0) readonly buffer Joints {
    mat4 joint_matrices[];
};

void main() {
    mat4 skin = inWeights.x * joint_matrices[inJoints.x]
              + inWeights.y * joint_matrices[inJoints.y]
              + inWeights.z * joint_matrices[inJoints.z]
              + inWeights.w * joint_matrices[inJoints.w];
    vec4 skinned = skin * vec4(inPosition, 1.0);
    gl_Position = pc.mvp * skinned;
    fragColor = vec3(1.0, 1.0, 1.0);
}
