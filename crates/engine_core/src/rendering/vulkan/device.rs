use ash::vk::{self, PhysicalDeviceMemoryProperties, PhysicalDeviceProperties};

use crate::rendering::vulkan::queue::QueueFamily;

#[derive(Debug, Clone)]
pub struct PhysicalDevice {
    pub handle: vk::PhysicalDevice,
    pub properties: PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub memory_properties: PhysicalDeviceMemoryProperties,
    pub queue_families: Vec<QueueFamily>,
}
