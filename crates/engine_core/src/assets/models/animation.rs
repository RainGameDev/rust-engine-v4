use anyhow::Result;
use macros::{Asset, Component, fixed_update};
use nalgebra::{Matrix4, UnitQuaternion, Vector3};

use crate::{
    assets::core::handle::Handle,
    ecs::{
        components::engine_components::transform::Transform, query::query::Query,
        systems::param::Assets,
    },
};

#[derive(Debug, Clone)]
pub enum Interpolation {
    Linear,
    Step,
    CubicSpline,
}

#[derive(Debug, Clone)]
pub enum TrackData {
    Translation(Vec<Vector3<f32>>),
    Rotation(Vec<UnitQuaternion<f32>>),
    Scale(Vec<Vector3<f32>>),
}

/// One animated property on one node.
#[derive(Debug, Clone)]
pub struct Track {
    // index into the Skeleton's joints, not a raw gltf node index
    pub target_joint: usize,
    pub times: Vec<f32>,
    pub data: TrackData,
    pub interpolation: Interpolation,
}

#[derive(Debug, Clone, Asset)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<Track>,
}

/// Joint hierarchy for a skinned mesh .
#[derive(Debug, Clone, Asset)]
pub struct Skeleton {
    // None = root joint
    pub joint_parents: Vec<Option<usize>>,
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
    pub joint_names: Vec<String>,
}

#[derive(Component, Debug, Clone)]
pub struct SkinnedMesh {
    pub skeleton: Handle<Skeleton>,
    pub joint_matrices: Vec<Matrix4<f32>>,
}

#[derive(Component, Debug, Clone)]
pub struct AnimationPlayer {
    pub clip: Handle<AnimationClip>,
    pub time: f32,
    pub speed: f32,
    pub looping: bool,
}

#[fixed_update]
fn advance_animations(
    mut players: Query<(&mut AnimationPlayer, &mut SkinnedMesh)>,
    clips: Assets<AnimationClip>,
    skeletons: Assets<Skeleton>,
    delta: f32,
) -> Result<()> {
    for (player, skinned) in players.iter() {
        let Some(clip) = clips.get(player.clip) else {
            continue;
        };
        let Some(skeleton) = skeletons.get(skinned.skeleton) else {
            continue;
        };

        player.time += delta * player.speed;
        if player.looping && player.time > clip.duration {
            player.time %= clip.duration.max(0.0001);
        }

        let mut local_transforms: Vec<Transform> = (0..skeleton.joint_parents.len())
            .map(|_| Transform::identity())
            .collect();

        for track in &clip.tracks {
            sample_track(
                track,
                player.time,
                &mut local_transforms[track.target_joint],
            );
        }

        // walk joint hierarchy, compose parent * local for each joint, apply inverse bind
        let mut global_transforms = vec![nalgebra::Matrix4::identity(); local_transforms.len()];
        for joint in 0..local_transforms.len() {
            let local = local_transforms[joint].to_matrix();
            global_transforms[joint] = match skeleton.joint_parents[joint] {
                Some(parent) => global_transforms[parent] * local,
                None => local,
            };
        }

        skinned.joint_matrices = (0..local_transforms.len())
            .map(|i| global_transforms[i] * skeleton.inverse_bind_matrices[i])
            .collect();
    }

    Ok(())
}

fn sample_track(track: &Track, time: f32, out: &mut Transform) {
    let idx = track.times.partition_point(|&t| t <= time);
    let (i0, i1, t) = if idx == 0 {
        (0, 0, 0.0)
    } else if idx >= track.times.len() {
        let last = track.times.len() - 1;
        (last, last, 0.0)
    } else {
        let t0 = track.times[idx - 1];
        let t1 = track.times[idx];
        let t = if t1 > t0 {
            (time - t0) / (t1 - t0)
        } else {
            0.0
        };
        (idx - 1, idx, t)
    };

    match &track.data {
        TrackData::Translation(values) => {
            out.position = values[i0].lerp(&values[i1], t);
        }
        TrackData::Rotation(values) => {
            out.rotation = values[i0].slerp(&values[i1], t);
        }
        TrackData::Scale(values) => {
            out.scale = values[i0].lerp(&values[i1], t);
        }
    }
}
