use anyhow::Result;
use ash::vk;
use egui::{ClippedPrimitive, Color32, Context, TextureId};
use egui_ash_renderer::{DynamicRendering, Options, Renderer, SamplerOptions};
use egui_winit::State;
use std::sync::Arc;
use winit::window::Window;

use std::sync::Mutex;

use crate::rendering::{
    rendering_settings::ImageSettings,
    vulkan::{context::VulkanRenderingContext, swapchain::VulkanSwapchain},
};

#[derive(Clone)]
pub struct UIRenderer {
    pub state: Arc<Mutex<State>>,
    pub renderer: Arc<Mutex<Renderer>>,
    pub context: Context,
    pub window: Arc<Window>,

    pub cached_primitives: Vec<ClippedPrimitive>,
    pub pending_texture_frees: Vec<TextureId>,
}

impl UIRenderer {
    pub fn new(
        context: VulkanRenderingContext,
        swapchain: &VulkanSwapchain,
        window: Arc<Window>,
        image_settings: ImageSettings,
    ) -> Result<Self> {
        let renderer = Renderer::with_default_allocator(
            &context.instance,
            context.physical_device.handle,
            context.device.clone(),
            DynamicRendering {
                color_attachment_format: swapchain.format,
                depth_attachment_format: Some(swapchain.depth.format),
                stencil_attachment_format: None,
            },
            Options {
                srgb_framebuffer: true,
                sampler_options: sampler_options_from(image_settings),
                ..Default::default()
            },
        )?;
        let context = Context::default();

        // TODO: make style
        // context.set_style(style);

        let state = State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            renderer: Arc::new(Mutex::new(renderer)),
            context,
            window,

            cached_primitives: Vec::new(),
            pending_texture_frees: Vec::new(),
        })
    }

    /// Updates the sampler options used for egui managed textures, re-sampling
    /// already uploaded textures so the change applies immediately.
    pub fn update_sampler_options(&mut self, image_settings: ImageSettings) {
        let mut renderer = self.renderer.lock().unwrap();
        let options = sampler_options_from(image_settings);
        renderer.set_sampler_options(options);
        renderer.update_samplers().unwrap();
    }
}

fn sampler_options_from(image_settings: ImageSettings) -> SamplerOptions {
    SamplerOptions {
        filter: image_settings.filter_mode,
        // egui requires CLAMP_TO_EDGE: solid UI colors are drawn from a single white
        // texel at the atlas corner (WHITE_UV == (0,0)). With REPEAT that corner sample
        // blends the white texel with transparent neighbours under LINEAR filtering,
        // making windows/buttons appear see-through.
        address_mode: vk::SamplerAddressMode::CLAMP_TO_EDGE,
        anisotropy_enabled: image_settings.anisotropy_enabled,
        anisotropy_amount: image_settings.anisotropy_amount,
        mipmap_mode: image_settings.mip_map_mode,
    }
}

pub const DARK_BG: Color32 = Color32::from_rgb(18, 18, 18);
pub const PANEL_BG: Color32 = Color32::from_rgb(24, 24, 24);
pub const HEADER_BG: Color32 = Color32::from_rgb(30, 30, 30);
pub const ROW_ALT: Color32 = Color32::from_rgb(28, 28, 28);
pub const DIV_COL: Color32 = Color32::from_rgb(60, 60, 60);
pub const TEXT_COL: Color32 = Color32::WHITE;
pub const DIM_COL: Color32 = Color32::from_rgb(170, 170, 170);
pub const SEL_BG: Color32 = Color32::from_rgb(40, 80, 140);
pub const HOVER_BG: Color32 = Color32::from_rgb(38, 38, 50);
pub const DRAG_SIZE: egui::Vec2 = egui::vec2(60.0, 20.0);
pub const VERTICAL_THICK_DRAG_SIZE: egui::Vec2 = egui::vec2(60.0, 40.0);
pub const LABEL_WIDTH: f32 = 100.0;
