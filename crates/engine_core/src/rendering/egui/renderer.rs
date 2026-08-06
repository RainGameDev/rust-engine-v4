use anyhow::Result;
use egui::{ClippedPrimitive, Color32, Context, TextureId};
use egui_ash_renderer::{DynamicRendering, Options, Renderer};
use egui_winit::State;
use std::sync::Arc;
use winit::window::Window;

use std::sync::Mutex;

use crate::rendering::vulkan::{context::VulkanRenderingContext, swapchain::VulkanSwapchain};

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
