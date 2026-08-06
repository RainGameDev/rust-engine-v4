use std::{collections::HashMap, path::Path};

use anyhow::Result;
use ash::vk::CommandPool;
use gltf::{Document, animation::util::ReadOutputs};
use nalgebra::{Matrix4, UnitQuaternion, Vector3};

use crate::{
    assets::models::animation::{AnimationClip, Interpolation, Skeleton, Track, TrackData},
    log_error,
    rendering::{
        core::{model::GpuMesh, vertex::Vertex},
        vulkan::context::VulkanRenderingContext,
    },
};

/// Everything loaded from a single gltf file.
pub struct LoadedGltf {
    pub meshes: Vec<GpuMesh>,
    pub skeletons: Vec<Skeleton>,
    pub animations: Vec<AnimationClip>,
}

/// Loads a gltf file from `path` and loads it into several `GpuMesh`s for rendering,
/// plus any `Skeleton`s and `AnimationClip`s it contains.
/// Takes in a `context` and `command_pool` to create the buffers needed for the mesh.
pub fn load_gltf_file(
    path: &Path,
    context: &VulkanRenderingContext,
    command_pool: CommandPool,
) -> Result<LoadedGltf> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("model path is not valid UTF-8: {:?}", path))?;
    let (gltf, buffers, _images) = gltf::import(path_str)?;

    let skeletons = load_skeletons(&gltf, &buffers);
    let animations = load_animations(&gltf, &buffers, &skeletons);

    let mut meshes: Vec<GpuMesh> = Vec::new();

    let mut all_triangles: Vec<[Vector3<f32>; 3]> = Vec::new();

    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let Some(positions) = reader.read_positions().map(|p| p.collect::<Vec<_>>()) else {
                log_error!("Skipping primitive in {:?}: no position data", path);
                continue;
            };

            let indices = match reader.read_indices() {
                Some(i) => i.into_u32().collect::<Vec<_>>(),
                None => (0..positions.len() as u32).collect::<Vec<_>>(),
            };

            let normals = match reader.read_normals() {
                Some(n) => n.collect::<Vec<_>>(),
                None => compute_normals(&positions, &indices),
            };

            let uv = match reader.read_tex_coords(0) {
                Some(t) => t.into_f32().collect::<Vec<_>>(),
                None => vec![[0.0, 0.0]; positions.len()],
            };

            let tangents = match reader.read_tangents() {
                Some(t) => t.collect::<Vec<_>>(),
                None => compute_tangents(&positions, &normals, &uv, &indices),
            };

            let joints: Vec<[u16; 4]> = match reader.read_joints(0) {
                Some(j) => j.into_u16().collect(),
                None => vec![[0, 0, 0, 0]; positions.len()],
            };

            let weights: Vec<[f32; 4]> = match reader.read_weights(0) {
                Some(w) => w.into_f32().collect(),
                None => vec![[1.0, 0.0, 0.0, 0.0]; positions.len()],
            };
            let vertices: Vec<Vertex> = positions
                .iter()
                .zip(normals.iter())
                .zip(uv.iter())
                .zip(tangents.iter())
                .zip(joints.iter())
                .zip(weights.iter())
                .map(|(((((pos, norm), uv), _tan), joints), weights)| Vertex {
                    position: *pos,
                    normal: *norm,
                    uv: *uv,
                    joints: *joints,
                    weights: *weights,
                    // color: [1.0, 1.0, 1.0],
                    // tangent: *tan,
                })
                .collect();

            let vertex_buffer = context.create_vertex_buffer(vertices.as_slice(), command_pool)?;
            let index_buffer = context.create_index_buffer(&indices, command_pool)?;

            // Collect the vertices
            for chunk in indices.chunks(3) {
                if let [i0, i1, i2] = *chunk {
                    let p = |i: u32| {
                        let p = positions[i as usize];
                        Vector3::new(p[0], p[1], p[2])
                    };
                    all_triangles.push([p(i0), p(i1), p(i2)]);
                }
            }

            let material_name = primitive
                .material()
                .name()
                .unwrap_or("material")
                .to_string();

            // Collect the mesh
            meshes.push(GpuMesh {
                vertex_buffer: vertex_buffer.0,
                vertex_buffer_memory: vertex_buffer.1,
                index_buffer: index_buffer.0,
                index_buffer_memory: index_buffer.1,
                index_count: indices.len() as u32,
                material_name,
            });
        }
    }

    Ok(LoadedGltf {
        meshes,
        skeletons,
        animations,
    })
}

/// Builds a `Skeleton` for every skin in the gltf file.
pub fn load_skeletons(gltf: &Document, buffers: &[gltf::buffer::Data]) -> Vec<Skeleton> {
    let mut parents: HashMap<usize, usize> = HashMap::new();
    for node in gltf.nodes() {
        for child in node.children() {
            parents.insert(child.index(), node.index());
        }
    }

    gltf.skins()
        .map(|skin| {
            let joint_nodes: Vec<usize> = skin.joints().map(|n| n.index()).collect();
            let joint_slot: HashMap<usize, usize> = joint_nodes
                .iter()
                .enumerate()
                .map(|(i, &n)| (n, i))
                .collect();

            let inverse_bind_matrices: Vec<Matrix4<f32>> = match skin
                .reader(|buffer| Some(&buffers[buffer.index()]))
                .read_inverse_bind_matrices()
            {
                Some(iter) => iter
                    .map(|m| Matrix4::from_column_slice(m.as_flattened()))
                    .collect(),
                // missing inverse bind matrices default to identity
                None => vec![Matrix4::identity(); joint_nodes.len()],
            };

            let joint_names: Vec<String> = joint_nodes
                .iter()
                .map(|&n| {
                    gltf.nodes()
                        .nth(n)
                        .and_then(|node| node.name())
                        .unwrap_or("joint")
                        .to_string()
                })
                .collect();

            let joint_parents: Vec<Option<usize>> = joint_nodes
                .iter()
                .map(|&node_index| {
                    let mut current = node_index;
                    loop {
                        match parents.get(&current) {
                            Some(&parent) => {
                                if let Some(&slot) = joint_slot.get(&parent) {
                                    return Some(slot);
                                }
                                current = parent;
                            }
                            // reached the root without finding a joint parent
                            None => return None,
                        }
                    }
                })
                .collect();

            Skeleton {
                joint_parents,
                inverse_bind_matrices,
                joint_names,
                joint_nodes,
            }
        })
        .collect()
}

/// Loads GLTF file animations into animation clips.
pub fn load_animations(
    gltf: &Document,
    buffers: &[gltf::buffer::Data],
    skeletons: &[Skeleton],
) -> Vec<AnimationClip> {
    let node_to_joint: HashMap<usize, usize> = skeletons
        .first()
        .map(|skeleton| {
            skeleton
                .joint_nodes
                .iter()
                .enumerate()
                .map(|(slot, &node)| (node, slot))
                .collect()
        })
        .unwrap_or_default();

    gltf.animations()
        .map(|anim| {
            let mut tracks = Vec::new();
            let mut duration = 0.0f32;

            // for the animations in the file
            for channel in anim.channels() {
                // read the animation buffer & length
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));
                let times: Vec<f32> = reader.read_inputs().unwrap().collect();
                duration = duration.max(times.last().copied().unwrap_or(0.0));

                // remap the gltf node index to a Skeleton joint index
                let Some(target_joint) =
                    node_to_joint.get(&channel.target().node().index()).copied()
                else {
                    // the track animates a node that isn't a skin joint, skip it
                    continue;
                };

                // convet the gltf interp to mine.
                let interpolation = match channel.sampler().interpolation() {
                    gltf::animation::Interpolation::Linear => Interpolation::Linear,
                    gltf::animation::Interpolation::Step => Interpolation::Step,
                    gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
                };

                // read the data
                let data = match reader.read_outputs().unwrap() {
                    ReadOutputs::Translations(t) => {
                        TrackData::Translation(t.map(Vector3::from).collect())
                    }
                    ReadOutputs::Rotations(r) => TrackData::Rotation(
                        r.into_f32()
                            .map(|[x, y, z, w]| {
                                UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                                    w, x, y, z,
                                ))
                            })
                            .collect(),
                    ),
                    ReadOutputs::Scales(s) => TrackData::Scale(s.map(Vector3::from).collect()),
                    ReadOutputs::MorphTargetWeights(_) => continue, // skip for now
                };

                // push to the track
                tracks.push(Track {
                    target_joint,
                    times,
                    data,
                    interpolation,
                });
            }

            // create a new clip
            AnimationClip {
                name: anim.name().unwrap_or("unnamed").to_string(),
                duration,
                tracks,
            }
        })
        .collect()
}

fn compute_tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tex_coords: &[[f32; 2]],
    indices: &[u32],
) -> Vec<[f32; 4]> {
    let mut tan = vec![Vector3::new(0.0f32, 0.0, 0.0); positions.len()];
    let mut bitan = vec![Vector3::new(0.0f32, 0.0, 0.0); positions.len()];
    for tri in indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if i0 >= positions.len() || i1 >= positions.len() || i2 >= positions.len() {
            continue;
        }
        let p0 = Vector3::from(positions[i0]);
        let p1 = Vector3::from(positions[i1]);
        let p2 = Vector3::from(positions[i2]);
        let uv0 = tex_coords[i0];
        let uv1 = tex_coords[i1];
        let uv2 = tex_coords[i2];
        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let duv1 = [uv1[0] - uv0[0], uv1[1] - uv0[1]];
        let duv2 = [uv2[0] - uv0[0], uv2[1] - uv0[1]];
        let denom = duv1[0] * duv2[1] - duv2[0] * duv1[1];
        if denom.abs() < 1e-12 {
            continue;
        }
        let f = 1.0 / denom;
        let t = (edge1 * duv2[1] - edge2 * duv1[1]) * f;
        let b = (edge2 * duv1[0] - edge1 * duv2[0]) * f;
        for i in [i0, i1, i2] {
            tan[i] += t;
            bitan[i] += b;
        }
    }
    positions
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let n = Vector3::from(normals[i]);
            let mut t = tan[i];
            t -= n * n.dot(&t);
            let t = if t.norm_squared() > 1e-12 {
                t.normalize()
            } else {
                let fallback = if n.x.abs() < 0.9 {
                    Vector3::new(1.0, 0.0, 0.0)
                } else {
                    Vector3::new(0.0, 1.0, 0.0)
                };
                (fallback - n * n.dot(&fallback)).normalize()
            };
            let handedness = if n.cross(&t).dot(&bitan[i]) < 0.0 {
                -1.0
            } else {
                1.0
            };
            [t.x, t.y, t.z, handedness]
        })
        .collect()
}

/// Computes per-vertex normals by accumulating face normals over the index list.
/// Used as a fallback when a glTF primitive ships without a NORMAL attribute.
fn compute_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![Vector3::new(0.0f32, 0.0, 0.0); positions.len()];
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if a >= positions.len() || b >= positions.len() || c >= positions.len() {
            continue;
        }
        let pa = Vector3::from(positions[a]);
        let pb = Vector3::from(positions[b]);
        let pc = Vector3::from(positions[c]);
        let face = (pb - pa).cross(&(pc - pa));
        normals[a] += face;
        normals[b] += face;
        normals[c] += face;
    }
    normals
        .into_iter()
        .map(|n| {
            if n.norm_squared() > 1e-12 {
                let n = n.normalize();
                [n.x, n.y, n.z]
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect()
}
