use ash::vk;
use egui::TextureId;
use macros::Resource;

#[derive(Resource, Debug, Clone)]
pub struct ViewportTexture(pub TextureId);

#[derive(Resource, Debug, Clone, Copy)]
pub struct ViewportSize {
    // viewport top-left position
    pub logical_x: f32,
    pub logical_y: f32,
    // egui logical points
    pub logical_width: f32,
    pub logical_height: f32,
    // physical pixels (pixels_per_point * logical * supersample)
    pub pixel_width: f32,
    pub pixel_height: f32,
}

impl ViewportSize {
    pub fn new(logical_w: f32, logical_h: f32) -> Self {
        Self {
            logical_x: 0.0,
            logical_y: 0.0,
            logical_width: logical_w,
            logical_height: logical_h,
            pixel_width: logical_w,
            pixel_height: logical_h,
        }
    }

    pub fn to_extent(&self) -> vk::Extent2D {
        vk::Extent2D {
            width: self.pixel_width as u32,
            height: self.pixel_height as u32,
        }
    }

    pub fn aspect_logical(&self) -> f32 {
        if self.logical_height == 0.0 {
            1.0
        } else {
            self.logical_width / self.logical_height
        }
    }
}

impl Default for ViewportSize {
    fn default() -> Self {
        Self {
            logical_x: 0.0,
            logical_y: 0.0,
            logical_width: 960.0,
            logical_height: 540.0,
            pixel_width: 960.0,
            pixel_height: 540.0,
        }
    }
}
