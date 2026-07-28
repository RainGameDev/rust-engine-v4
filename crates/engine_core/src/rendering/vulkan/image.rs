use ash::vk::{self, DeviceMemory, Format, ImageView};

pub struct Image {
    pub format: Format,
    pub image: vk::Image,
    pub image_view: ImageView,
    pub memory: DeviceMemory,
}

#[derive(Clone, Copy)]
pub struct ImageLayoutState {
    pub access_mask: vk::AccessFlags,
    pub layout: vk::ImageLayout,
    pub stage_mask: vk::PipelineStageFlags,
    pub queue_family_index: u32,
}

#[derive(Clone, Copy)]
pub struct ImageLayouts {
    pub undefined: ImageLayoutState,
    pub renderable: ImageLayoutState,
    pub present: ImageLayoutState,
}

impl Default for ImageLayouts {
    fn default() -> Self {
        let undefined = ImageLayoutState {
            layout: vk::ImageLayout::UNDEFINED,
            access_mask: vk::AccessFlags::empty(),
            stage_mask: vk::PipelineStageFlags::TOP_OF_PIPE,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        };
        let renderable = ImageLayoutState {
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        };

        let present = ImageLayoutState {
            layout: vk::ImageLayout::PRESENT_SRC_KHR,
            access_mask: vk::AccessFlags::empty(),
            stage_mask: vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        };

        Self {
            undefined,
            renderable,
            present,
        }
    }
}
