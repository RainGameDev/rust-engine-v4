use std::ffi::{CStr, c_void};

use ash::{Entry, Instance, vk};

/// The standard Khronos validation layer.
pub const VALIDATION_LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

/// The names of the instance layers the current Vulkan loader can see.
pub fn enumerate_instance_layer_names(entry: &Entry) -> Vec<String> {
    let layers = unsafe {
        entry
            .enumerate_instance_layer_properties()
            .unwrap_or_default()
    };
    layers
        .iter()
        .map(|layer| {
            let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
            name.to_string_lossy().into_owned()
        })
        .collect()
}

/// Whether `VK_LAYER_KHRONOS_validation` is present on the system.
pub fn is_validation_layer_available(entry: &Entry) -> bool {
    let validation_name = VALIDATION_LAYER_NAME.to_str().unwrap();
    enumerate_instance_layer_names(entry)
        .iter()
        .any(|name| name == validation_name)
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let message = if callback_data.is_null() {
        None
    } else {
        let data = unsafe { &*callback_data };
        Some(unsafe { CStr::from_ptr(data.p_message) }.to_string_lossy())
    };
    let message = message.as_deref().unwrap_or("(unknown vulkan message)");

    if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        crate::log_error!("[Vulkan Validation] {}", message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        crate::log_warn!("[Vulkan Validation] {}", message);
    } else {
        crate::log_debug!("[Vulkan Validation] {}", message);
    }

    vk::FALSE
}

/// Holds the debug utils extension and its messenger.
#[derive(Clone)]
pub struct DebugUtils {
    /// The debug utils extension wrapper.
    pub debug_utils: ash::ext::debug_utils::Instance,
    /// The messenger receiving validation messages.
    pub messenger: vk::DebugUtilsMessengerEXT,
}

impl DebugUtils {
    /// Creates a debug utils messenger for the given instance.
    /// Returns `None` if the validation layer is unavailable.
    pub fn new(entry: &Entry, instance: &Instance) -> Option<Self> {
        let validation_name = VALIDATION_LAYER_NAME.to_str().unwrap();
        let available_layers = enumerate_instance_layer_names(entry);
        if !available_layers.iter().any(|name| name == validation_name) {
            let layers = if available_layers.is_empty() {
                "(none reported)".to_string()
            } else {
                available_layers.join(", ")
            };
            crate::log_warn!(
                reason: "VK_LOADER_LAYERS",
                "Validation layer '{}' is not available. Layers visible to the loader: {}",
                VALIDATION_LAYER_NAME.to_string_lossy(),
                layers
            );
            return None;
        }

        let debug_utils = ash::ext::debug_utils::Instance::new(entry, instance);
        let messenger_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        let messenger = unsafe {
            debug_utils
                .create_debug_utils_messenger(&messenger_create_info, None)
                .ok()?
        };

        crate::log_info!(
            "Vulkan validation layer '{}' enabled.",
            VALIDATION_LAYER_NAME.to_string_lossy()
        );

        Some(Self {
            debug_utils,
            messenger,
        })
    }

    /// The create info used both as the `p_next` of `InstanceCreateInfo` (so
    /// messages during instance creation are captured) and for the messenger.
    pub fn messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback))
    }
}
