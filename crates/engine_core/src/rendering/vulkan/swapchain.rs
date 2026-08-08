use std::sync::Arc;

use anyhow::Result;
use ash::vk::{self, Extent2D, Format, ImageView, SwapchainKHR};
use winit::window::Window;

use crate::rendering::vulkan::{context::VulkanRenderingContext, image::Image, surface::Surface};

/// A swapchain defined for vulkan
pub struct VulkanSwapchain {
    handle: SwapchainKHR,
    surface: Surface,
    context: Arc<VulkanRenderingContext>,
    pub desired_image_count: u32,
    /// How should the swapchain be formatted.
    pub format: Format,
    /// The size of the swapchain.
    pub extent: Extent2D,
    /// Vector of ImageView objects.
    pub views: Vec<ImageView>,
    /// Vector of image objects.
    pub images: Vec<vk::Image>,
    /// Reference to the window the swapchain renders on.
    pub window: Arc<Window>,
    /// Depth handling.
    pub depth: Image,

    // Does the swapchain need remaking?
    pub is_dirty: bool,
}

impl VulkanSwapchain {
    /// Creates a vulkan swapchain
    pub fn new(context: Arc<VulkanRenderingContext>, window: Arc<Window>) -> Result<Self> {
        let surface = context.create_surface(&window)?;
        let format = vk::Format::B8G8R8A8_SRGB;
        let depth_format = vk::Format::D24_UNORM_S8_UINT;
        let extent = if surface.capabilities.current_extent.width != u32::MAX {
            surface.capabilities.current_extent
        } else {
            vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height,
            }
        };
        let image_count = (surface.capabilities.min_image_count + 1).clamp(
            surface.capabilities.min_image_count,
            if surface.capabilities.max_image_count != 0 {
                surface.capabilities.max_image_count
            } else {
                u32::MAX
            },
        );

        let depth = Image {
            format: depth_format,
            image: vk::Image::null(),
            image_view: vk::ImageView::null(),
            memory: vk::DeviceMemory::null(),
        };

        Ok(Self {
            desired_image_count: image_count,
            format,
            extent,
            views: Vec::new(),
            images: Vec::new(),
            handle: Default::default(),
            surface,
            window,
            context,
            is_dirty: true,
            depth,
        })
    }

    /// Acquires the next image in the swapchain
    pub fn acquire_next_image(&mut self, image_available_semaphore: vk::Semaphore) -> Result<u32> {
        let (image_index, is_suboptimal) = unsafe {
            self.context.swapchain_extension.acquire_next_image(
                self.handle,
                u64::MAX,
                image_available_semaphore,
                vk::Fence::null(),
            )?
        };

        if is_suboptimal {
            self.is_dirty = true;
        }

        Ok(image_index)
    }

    /// Presents an image to the renderer
    pub fn present_image(
        &mut self,
        image_index: u32,
        render_finished_semaphore: vk::Semaphore,
    ) -> Result<()> {
        let is_suboptimal = unsafe {
            self.context.swapchain_extension.queue_present(
                self.context.queues[self.context.queue_families.present as usize],
                &vk::PresentInfoKHR::default()
                    .wait_semaphores(&[render_finished_semaphore])
                    .swapchains(&[self.handle])
                    .image_indices(&[image_index]),
            )
        }?;

        if is_suboptimal {
            self.is_dirty = true;
        }

        Ok(())
    }

    /// Resizes the swapchain based on the window size
    pub fn resize(&mut self) -> Result<()> {
        let size = self.window.inner_size();
        self.extent = vk::Extent2D {
            width: size.width,
            height: size.height,
        };

        if self.extent.width == 0 || self.extent.height == 0 {
            return Ok(());
        }

        unsafe {
            self.context.device.device_wait_idle()?;
            self.surface.capabilities = self
                .context
                .surface_extension
                .get_physical_device_surface_capabilities(
                    self.context.physical_device.handle,
                    self.surface.handle,
                )?;

            self.desired_image_count = (self.surface.capabilities.min_image_count + 1).clamp(
                self.surface.capabilities.min_image_count,
                if self.surface.capabilities.max_image_count != 0 {
                    self.surface.capabilities.max_image_count
                } else {
                    u32::MAX
                },
            );
            let present_mode = {
                let modes = self
                    .context
                    .surface_extension
                    .get_physical_device_surface_present_modes(
                        self.context.physical_device.handle,
                        self.surface.handle,
                    )?;
                if modes.contains(&vk::PresentModeKHR::MAILBOX) {
                    vk::PresentModeKHR::MAILBOX
                } else {
                    vk::PresentModeKHR::FIFO
                }
            };

            let composite_alpha = {
                let flags = self.surface.capabilities.supported_composite_alpha;
                if flags.contains(vk::CompositeAlphaFlagsKHR::OPAQUE) {
                    vk::CompositeAlphaFlagsKHR::OPAQUE
                } else if flags.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
                    vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
                } else if flags.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
                    vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
                } else {
                    vk::CompositeAlphaFlagsKHR::INHERIT
                }
            };

            let mut ci = vk::SwapchainCreateInfoKHR::default()
                .surface(self.surface.handle)
                .min_image_count(self.desired_image_count)
                .image_format(self.format)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(self.extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                .composite_alpha(composite_alpha)
                .present_mode(present_mode)
                .clipped(true);

            if self.handle != vk::SwapchainKHR::null() {
                ci = ci.old_swapchain(self.handle);
            }

            let new_swapchain = self
                .context
                .swapchain_extension
                .create_swapchain(&ci, None)?;

            for image_view in self.views.drain(..) {
                self.context.device.destroy_image_view(image_view, None);
            }

            self.context
                .swapchain_extension
                .destroy_swapchain(self.handle, None);

            self.images.clear();
            self.handle = new_swapchain;
            self.images = self
                .context
                .swapchain_extension
                .get_swapchain_images(new_swapchain)?;

            for image in &self.images {
                self.views.push(self.context.create_image_view(
                    *image,
                    self.format,
                    vk::ImageAspectFlags::COLOR,
                )?);
            }
        }

        self.create_depth_image()?;
        self.is_dirty = false;
        Ok(())
    }

    /// Creates (or recreates) the depth image to match the current swapchain extent.
    fn create_depth_image(&mut self) -> Result<()> {
        if self.depth.image != vk::Image::null() {
            unsafe {
                self.context
                    .device
                    .destroy_image_view(self.depth.image_view, None);
                self.context.device.destroy_image(self.depth.image, None);
                self.context.device.free_memory(self.depth.memory, None);
            }
        }

        let (image, memory) = self.context.create_image(
            self.extent,
            self.depth.format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let image_view = self.context.create_image_view(
            image,
            self.depth.format,
            vk::ImageAspectFlags::DEPTH,
        )?;

        self.depth = Image {
            format: self.depth.format,
            image,
            image_view,
            memory,
        };

        Ok(())
    }
}
