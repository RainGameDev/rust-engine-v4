use std::path::Path;

use image::{GenericImageView, RgbaImage};
use walkdir::WalkDir;

use crate::{
    ecs::World,
    log_debug, log_warn,
    rendering::core::{
        model::{RawMesh, raw_mesh_to_gpu_mesh_with_texture},
        vertex::Vertex,
    },
    rendering::rendering_settings::RenderingSettings,
    rendering::vulkan::{VulkanRenderer, context::VulkanRenderingContext},
};

/// Creates a flat quad mesh suitable for sprite rendering.
/// The quad is centered at the origin at the given z depth.
pub fn sprite_quad(width: f32, height: f32, z: f32) -> RawMesh {
    let hw = width * 0.5;
    let hh = height * 0.5;

    let vertices = vec![
        Vertex {
            position: [-hw, -hh, z],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [hw, -hh, z],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [hw, hh, z],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [-hw, hh, z],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    RawMesh {
        vertices,
        indices,
        material_name: String::new(),
    }
}

/// Loads all images from a directory recursively,
/// creates a textured sprite quad mesh for each,
/// and stores it as an asset keyed by its relative path.
pub fn load_image_sprites(
    dir: &Path,
    world: &mut World,
    renderer: &VulkanRenderer,
    context: &VulkanRenderingContext,
    settings: &RenderingSettings,
) {
    for entry in WalkDir::new(dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                log_warn!(reason: "walk error", "{err}");
                continue;
            }
        };
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        let is_image = matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg");
        if !is_image {
            continue;
        }

        let Ok(img) = image::open(path) else {
            log_warn!(reason: "failed to open image", "{}", path.display());
            continue;
        };

        let (w, h) = img.dimensions();
        let rgba: RgbaImage = img.to_rgba8();
        let pixels = rgba.into_raw();

        let max_dim = w.max(h) as f32;
        let mesh = raw_mesh_to_gpu_mesh_with_texture(
            sprite_quad(w as f32 / max_dim, h as f32 / max_dim, -0.1),
            &pixels,
            w,
            h,
            renderer,
            context,
            settings,
        );

        match mesh {
            Ok(gpu_mesh) => {
                let relative = path
                    .strip_prefix(dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .into_owned();
                log_debug!(
                    "sprite loaded: path={}, tex_descriptor_set={}",
                    relative,
                    if gpu_mesh.texture_descriptor_set.is_some() {
                        "Some"
                    } else {
                        "None"
                    }
                );
                world.add_asset(gpu_mesh, relative);
            }
            Err(err) => {
                log_warn!(
                    reason: "failed to create sprite mesh",
                    "{}: {err:?}",
                    path.display()
                );
            }
        }
    }
}
