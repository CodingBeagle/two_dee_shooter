use std::borrow::Borrow;
use std::collections::HashSet;
use std::ffi::{ CString, CStr, c_void };
use std::ptr;

use ash::vk::{Handle, DeviceQueueCreateFlags};
use ash::{vk, Entry};

use beagle_glfw::*;

#[macro_use]
extern crate lazy_static;

static WIDTH: i32 = 800;
static HEIGHT: i32 = 600;

lazy_static! {
    static ref REQUIRED_EXTENSIONS: HashSet<String> = {
        let mut m = HashSet::new();
        m.insert(String::from("VK_KHR_swapchain"));
        m
    };
}

static mut VK_ENTRY: Option<ash::Entry> = None;
static mut VK_INSTANCE: Option<ash::Instance> = None;
static mut VK_DEVICE: Option<ash::Device> = None;

fn main() {
    unsafe {
        if glfwInit() == 0 {
            panic!("Failed to initialize GLFW.");
        }

        // Vulkan Ash related initialization
        // TODO: Read up more on this Entry::Linked called. It seems to load the Vulkan library by linking to it statically.
        // But how does this work, and what exactly does it do???
        VK_ENTRY = Some(Entry::linked());

        /*
            In order to initialize Vulkan, we need to create an instance.
            The instance is a connection between your application and the Vulkan library.

            To create an instance, you first have to fill out a struct with information about the application.
            A lot of information in Vulkan will be passed through structs instead of function parameters.

            The "ApplicationInfo" struct is technically optional, but giving the information may help the driver optimize some things for
            our application.
        */
        let application_name = ffi_string("2D Shooter");
        let engine_name = ffi_string("No Engine");

        let application_info = vk::ApplicationInfo {
            s_type: vk::StructureType::APPLICATION_INFO,
            p_application_name: application_name.as_ptr(),
            application_version: vk::make_api_version(1, 0, 0, 0),
            p_engine_name: engine_name.as_ptr(),
            engine_version: vk::make_api_version(1, 0, 0, 0),
            api_version: vk::API_VERSION_1_0,
            ..Default::default()
        };

        // vkInstanceCreateInfo is a required struct which tells the Vulkan driver which global extensions and validation layers we want to use.
        // Global meaning: They apply to the entire program and not a specific device.
        // We also specify our application info struct in this struct.
        let required_extensions = build_extensions();

        // For debug builds, I'll enable standard validation layers that comes bundled with the LunarG Vulkan SDK.
        // These standard validations comes bundled into a layer in the SDK called "VK_LAYER_KHRONOS_validation".
        let required_validation_layers = vec!(
            "VK_LAYER_KHRONOS_validation"
        );

        // Retrieve all available layers.
        // TODO: Probably I could transform available_layers to a list of strings to quickly compare against my required validation layers
        let available_layers = VK_ENTRY.as_ref().unwrap().enumerate_instance_layer_properties().expect("Failed to retrieve available layers.");

        for required_validation_layer in &required_validation_layers {
            let mut is_required_validation_layer_supported = false;

            for available_layer in &available_layers {
                // TODO: Is this an owned string that is being converted to??
                let layer_name = CStr::from_ptr(available_layer.layer_name.as_ptr()).to_str().expect("Failed to get string from available layer.");
                if layer_name == (*required_validation_layer) {
                    is_required_validation_layer_supported = true;
                }
            }

            if !is_required_validation_layer_supported {
                panic!("The required validation layer {} could not be found in the list of available layers.", required_validation_layer);
            }
        }

        let validation_layers_as_cstrings : Vec<CString> = required_validation_layers
            .iter()
            .map(|layer_name| {
                CString::new(*layer_name).unwrap()
            })
            .collect();

        let validation_layers_as_raw_pointers: Vec<*const i8> = validation_layers_as_cstrings
            .iter()
            .map(|x| x.as_ptr())
            .collect();

        let required_extensions_as_c_string: Vec<CString> = required_extensions.iter()
            .map(|x| CString::new(x.clone()).expect("Failed to create CString from string"))
            .collect();

        let required_extensions_pointer: Vec<*const i8> = required_extensions_as_c_string
            .iter()
            .map(|x| x.as_ptr())
            .collect();

        // The Debug Utils debug messenger requires a valid instance in order to be created. In order to enable debug callbacks when creating the instance,
        // You can instead pass a DebugUtilsMessengerCreateInfoEXT object pointer to the InstanceCreateInfo struct's p_next property.
        // TODO: Do I need to handle the lifetime of this instance debug messenger myself??
        let instance_debug_messenger = populate_debug_messenger_create_info();

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_application_info: &application_info,
            enabled_extension_count: required_extensions_pointer.len() as u32,
            pp_enabled_extension_names: required_extensions_pointer.as_ptr(),
            pp_enabled_layer_names: validation_layers_as_raw_pointers.as_ptr(),
            enabled_layer_count: required_validation_layers.len() as u32,
            p_next: &instance_debug_messenger as *const vk::DebugUtilsMessengerCreateInfoEXT as *const c_void,
            ..Default::default()
        };

        // Now everything is specified for Vulkan to create an instance
        // This instance should live for as long as the application lives.
        // Creating a VkInstance object initializes the Vulkan library.
        // Per-application state is stored in this object. Vulkan does NOT have any global state.
        VK_INSTANCE = Some(VK_ENTRY.as_ref().unwrap().create_instance(&create_info, None).expect("Failed to create Vulkan instance."));

        // In order to create a debug messenger, we have to call the function "vkCreateDebugUtilsMessengerEXT"
        // Since this is an extension function, it is not automatically loaded with Vulkan.
        // We have to load it ourselves
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(VK_ENTRY.as_ref().unwrap(), VK_INSTANCE.as_ref().unwrap());
        let debug_utils_messenger = setup_debug_messenger(&debug_utils_loader);

        // After creating a Vulkan instance, we need to select a physical graphics card that supports the features we need.
        let physical_devices = VK_INSTANCE.as_ref().unwrap().enumerate_physical_devices().expect("Failed to retrieve physical devices.");

        // GLFW was originally designed to create an OpenGL context, so we have to tell it not to
        // since we'll be using Vulkan.
        glfwWindowHint(GLFW_CLIENT_API as i32, GLFW_NO_API as i32);

        // Handling resized windows takes special care.
        // Disabled for now.
        glfwWindowHint(GLFW_RESIZABLE as i32, GLFW_FALSE as i32);

        let window_title = ffi_string("Two Dee Shooter");
        let mut main_window = glfwCreateWindow(
            WIDTH,
            HEIGHT,
            window_title.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut());       

        // If main_window is NULL, window creation failed for some reason.
        if main_window.is_null() {
            panic!("Failed to create window: {}", get_latest_glfw_error_description());
        }

        // In order to present visuals to the window, we need to create a VkSurfaceKHR object.
        // This object represents an abstract type of surface to present rendered images to.
        // While the object and its usage is platform agnostic, the creation isn't.
        // The creation depends on window system details, like a HWND and HMODULE.
        // There is a platform-specific addition to "VK_KHR_SURFACE" called "VK_KHR_win32_surface" that handles this.
        let surface_extension = ash::extensions::khr::Surface::new(VK_ENTRY.as_ref().unwrap(), VK_INSTANCE.as_ref().unwrap());

        let mut some_surface: u64 = 0;

        // TODO: I manually edited the bindings.rs file to simply have u64 handles for parameters. The bindgen generation is bonkers.
        // I'll have to figure out how to make that generation automatic, by modifying the types through the bindgen builder.
        // Perhaps I should also raise an issue on bindgen github?
        let result = glfwCreateWindowSurface(VK_INSTANCE.as_ref().unwrap().handle().as_raw(), main_window, ptr::null(), &mut some_surface);

        if result != 0 {
            panic!("Failed to create Window Surface!");
        }

        let the_surface = vk::SurfaceKHR::from_raw(some_surface);        

        // TODO: Do something nice here, like printing a list of all available physical devices.
        let mut selected_physical_device: Option<vk::PhysicalDevice> = None;
        for physical_device in physical_devices {
            if is_device_suitable(VK_INSTANCE.as_ref().unwrap(), the_surface, &surface_extension, physical_device) {
                selected_physical_device = Some(physical_device);
            }
        }

        if selected_physical_device.is_none() {
            panic!("Failed to select a physical device!");
        }

        // Time to create a logical device from our physical device!

        // In order to create a logical device, I need to supply information on queues I want to have created, as well as
        // Device features I want to use.
        let indices = find_queue_families(VK_INSTANCE.as_ref().unwrap(), the_surface, &surface_extension, selected_physical_device.unwrap());

        let mut family_indices: HashSet<u32> = HashSet::new();
        family_indices.insert(indices.graphics_family.unwrap());
        family_indices.insert(indices.present_family.unwrap());

        // I run through each family index that I need to create a queue for, and create its DeviceQueueCreateInfo struct.
        // The list of these DeviceQueueCreateInfo structs will be passed to DeviceCreateInfo struct, when creating the logical device and its
        // required queues.
        let mut queues_to_create: Vec<vk::DeviceQueueCreateInfo> = vec!();

        // Vulkan requires that you assign priorities to queues, in order to influence the scheduling of command buffer execution.
        // The priority is specified using a floating point number between 0.0 and 1.0.
        // TODO: Read up more on this scheduling mechanism
        let queue_priority: f32 = 1.0;

        for family_index in family_indices {
            let queue_create_info = vk::DeviceQueueCreateInfo {
                s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
                queue_family_index: family_index,
                queue_count: 1,
                p_queue_priorities: &queue_priority,
                ..Default::default()
            };

            queues_to_create.push(queue_create_info);
        }

        // We also need to supply information about device features we want.
        // Right now, I don't need anything in particular, so I'll leave the struct with default values.
        let device_features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };

        // Required device extensions
        let required_device_extensions: Vec<String> = REQUIRED_EXTENSIONS.clone().into_iter().collect();
        let required_device_extensions_cstrings = strings_to_cstrings(required_device_extensions);
        let required_device_extensions_raw_pointers = strings_to_raw_pointers(&required_device_extensions_cstrings);

        // Now I create the logical device
        // Qeues will be created automatically with the logical device.
        let logical_device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_queue_create_infos: queues_to_create.as_ptr(),
            queue_create_info_count: queues_to_create.len() as u32,
            p_enabled_features: &device_features,
            // Previous implementations of Vulkan made a distinction between instance and device specific validation layers,
            // but this is no longer the case. "enabled_layer_count" and "pp_enabled_layer_names" are ignored by up-to-date implementations.
            // However, it's a good idea to set the anyways to be compatible with older implementations.
            enabled_layer_count: required_validation_layers.len() as u32,
            pp_enabled_layer_names: validation_layers_as_raw_pointers.as_ptr(),
            enabled_extension_count: required_device_extensions_raw_pointers.len() as u32,
            pp_enabled_extension_names: required_device_extensions_raw_pointers.as_ptr(),
            ..Default::default()
        };

        VK_DEVICE = Some( 
            match VK_INSTANCE.as_ref().unwrap().create_device(selected_physical_device.unwrap(), &logical_device_create_info, None) {
                Ok(physical_device) => physical_device,
                Err(err) => panic!("Failed to create physical device: {}", err)
            });

        // Now that we have a logical device, we can retrieve the queue we need.
        // Right now, we need the queue that supports presentation.
        let device_presentation_queue = VK_DEVICE.as_ref().unwrap().get_device_queue(indices.present_family.unwrap(), 0);

        while glfwWindowShouldClose(main_window) == 0 {
            glfwPollEvents();
        }

        // Delete the logical device
        VK_DEVICE.as_ref().unwrap().destroy_device(None);

        // Clean up the debug messenger
        // Destroying the debug messenger must be done before the Vulkan instance is destroyed.
        // TODO: Does Ash handle any of these calls in Drop implementations of the structs??
        debug_utils_loader.destroy_debug_utils_messenger(debug_utils_messenger, None);

        // We destroy the KHR Surfance
        surface_extension.destroy_surface(the_surface, None);

        // Before we terminate the application, we destroy the Vulkan instance.
        VK_INSTANCE.as_ref().unwrap().destroy_instance(None);

        glfwDestroyWindow(main_window);

        // Before terminating your application, you should terminate the GLFW library if it has been initialized.
        // If you don't global system settings changed by GLFW might not be restored properly.
        glfwTerminate();
    }
}

unsafe fn create_swap_chain(surface_extensions: &ash::extensions::khr::Surface, surface: vk::SurfaceKHR, device: vk::PhysicalDevice, window: *mut GLFWwindow) {
    let swap_chain_support_details = query_swapchain_support(surface_extensions, surface, device);

    let surface_format = choose_swap_surface_format(swap_chain_support_details.formats);
    let present_mode = choose_swap_present_mode(swap_chain_support_details.presentModes);
    let extent = choose_swap_extent(window, swap_chain_support_details.capabilities);

    // We need to decide how many images we would like to have in the swap chain.
    // capabilities.min_image_count specifies the minimum number of images the implementation requires to function.
    // However, it is recommended to request at least one more image than the minimum, in order to avoid having to wait for the driver to complete
    // internal operations before we can aquire another image to render to.
    let mut image_count = swap_chain_support_details.capabilities.min_image_count + 1;

    // However, we still need to make sure that we do not exceed the maximum supported image count.
    if swap_chain_support_details.capabilities.max_image_count > 0 && image_count > swap_chain_support_details.capabilities.max_image_count {
        image_count = swap_chain_support_details.capabilities.max_image_count;
    }

    // The "image_array_layers" property specifies the amount of layers each image consists of.
    // This is always "1", unless you are developing a stereoscopic 3D application.
    // The "image_usage" property specificies what operations we'll use the images in the swap chain for.
    // "COLOR_ATTACHMENT" means we render to them directly.
    // "pre_transform" can be used to specify certain transforms that should be applied to images in the swap chain.
    // To specify that you do not want any transformation, simply specify the current transformation.
    // "composite_alpha" can be used to specify if the alpha channel should be used for blending with other windows in the window system.
    // You'll almost always want to simply ignore the alpha channel, which is "vk::CompositeAlphaFlagsKHR::OPAQUE".
    // TODO: Read up on "old_swapchain", complex topic regarding recreation of swap_chains in events such as resizing of window.
    let swap_chain_create_info = vk::SwapchainCreateInfoKHR {
        s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
        surface: surface,
        min_image_count: image_count,
        image_format: surface_format.format,
        image_color_space: surface_format.color_space,
        image_extent: extent,
        image_array_layers: 1,
        image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
        pre_transform: swap_chain_support_details.capabilities.current_transform,
        composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
        present_mode: present_mode,
        clipped: vk::TRUE,
        old_swapchain: vk::SwapchainKHR::null(),
        ..Default::default()
    };

    let swapchain = ash::extensions::khr::Swapchain::new(VK_INSTANCE.as_ref().unwrap(), VK_DEVICE.as_ref().unwrap());
}

// VkSurfaceFormatKHR contains two properties:
// - format
// - colorSpace
// Format describes the color channels and types.
// colorSpace indicates if the SRGB color space is supported or not.
// Right now, I will always prefer the format B8G8R8A8_SRGB with SRGB colorspace.
fn choose_swap_surface_format(available_formats: Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
    for surface_format in &available_formats {
        if surface_format.format == vk::Format::B8G8R8A8_SRGB && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
            println!("Picked preferred format and colorspace: B8G8R8A8_SRGB & SRGB");
            return *surface_format;
        }
    }

    // In case the preferred case isn't available, we'll pick whatever is the first available format.
    *available_formats.first().unwrap()
}

fn choose_swap_present_mode(available_present_modes: Vec<vk::PresentModeKHR>) -> vk::PresentModeKHR {
    for present_mode in &available_present_modes {
        // Currently prefer the MAILBOX present mode (similar to triple buffering)
        if *present_mode == vk::PresentModeKHR::MAILBOX {
            return *present_mode;
        }
    }

    // If triple buffering isn't available we will prefer FIFO.
    // This presentation mode is the only one guarenteed to be available.
    vk::PresentModeKHR::FIFO
}

// The swap extent is the resolution of the swap chain images.
// It's almost always equal to the resolution of the window that we're drawing to
// IN PIXELS.
// The range of possible resolutions is defined in the vk::SurfaceCapabilitiesKHR structure.
unsafe fn choose_swap_extent(window: *mut GLFWwindow, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    // If the width or height is not the maximum allowed value of u32,
    // This means that Vulkan has matched the resolution of the window
    // Otherwise, we need to pick the resolution that best matches the window within
    // the bounds of "minImageExtent" and "maxImageExtent", which is the range of
    // possible resolutions as determined by Vulkan.
    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    } else {
        let mut width: i32 = 0;
        let mut height: i32 = 0;

        glfwGetFramebufferSize(window, &mut width, &mut height);

        let actual_extent = vk::Extent2D {
            width: clamp(width as u32, capabilities.min_image_extent.width, capabilities.max_image_extent.width),
            height: clamp(height as u32, capabilities.min_image_extent.height, capabilities.max_image_extent.height),
        };

        actual_extent
    }
}

fn clamp(value: u32, min: u32, max: u32) -> u32 {
    if value < min {
        return min;
    } else if value > max {
        return max;
    }

    value
}

#[derive(Default)]
struct SwapChainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    presentModes: Vec<vk::PresentModeKHR>
}

unsafe fn query_swapchain_support(surface_extensions: &ash::extensions::khr::Surface, surface: vk::SurfaceKHR, device: vk::PhysicalDevice) -> SwapChainSupportDetails {
    let swapchain_support_details = SwapChainSupportDetails {
        capabilities: surface_extensions.get_physical_device_surface_capabilities(device, surface).unwrap(),
        formats: surface_extensions.get_physical_device_surface_formats(device, surface).unwrap(),
        presentModes: surface_extensions.get_physical_device_surface_present_modes(device, surface).unwrap()
    };

    swapchain_support_details
}

fn strings_to_cstrings(strings: Vec<String>) -> Vec<CString> {
    let wut: Vec<CString> = strings
        .iter()
        .map(|string| {
            CString::new(string.clone()).unwrap()
        })
        .collect();

    wut
}

unsafe fn strings_to_raw_pointers(strings: &Vec<CString>) -> Vec<*const i8> {
    strings
        .iter()
        .map(|string| string.as_ptr())
        .collect()
}

#[derive(Default)]
struct QueueFamilyIndices {
    graphics_family: Option<u32>,
    present_family: Option<u32>
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

unsafe fn find_queue_families(instance: &ash::Instance, surface: vk::SurfaceKHR, khr_extension: &ash::extensions::khr::Surface, physical_device: vk::PhysicalDevice) -> QueueFamilyIndices {
    let mut indices = QueueFamilyIndices::default();

    // Retrieve a list of queue families for a physical device
    // QueueFamiliyProperties contains details about the queue family, including the type of operations that are
    // Supported and the number of queues that can be created based on that family.
    // Right now, we need to find a queue that supports VK_QUEUE_GRAPHICS_BIT
    let queue_families = instance.get_physical_device_queue_family_properties(physical_device);

    let mut current_family_index: u32 = 0;
    for queue_family in queue_families {
        if queue_family.queue_flags & vk::QueueFlags::GRAPHICS == vk::QueueFlags::GRAPHICS {
            println!("Detected queue family supporting GRAPHICS");
            indices.graphics_family = Some(current_family_index);
        }

        // It is actually possible that the queue families supporting drawing commands and the ones supporting presentation do not overlap.
        // There, we need to store distinct indices for drawing and presentation queues.
        // Here, I query for presentation support.
        if khr_extension.get_physical_device_surface_support(physical_device, current_family_index, surface).is_ok() {
            indices.present_family = Some(current_family_index);
        }

        if indices.is_complete() {
            break;
        }

        current_family_index += 1;
    }

    indices
}

unsafe fn is_device_suitable(instance: &ash::Instance, surface: vk::SurfaceKHR, khr_extension: &ash::extensions::khr::Surface, device: vk::PhysicalDevice) -> bool {
    let device_properties = instance.get_physical_device_properties(device);
    let device_features = instance.get_physical_device_features(device);

    let extensions_supported = check_device_extension_support(instance, device);

    let mut swapchain_adequate = false;
    if extensions_supported {
        let swapchain_details = query_swapchain_support(&khr_extension, surface, device);
        swapchain_adequate = !swapchain_details.formats.is_empty() && !swapchain_details.presentModes.is_empty();
    }

    let device_name = CStr::from_ptr(device_properties.device_name.as_ptr());
    println!("Checking physical device: {}", device_name.to_str().expect("Failed to convert CStr to string!"));
    
    let selection_criteria = 
        (device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU && device_features.geometry_shader > 0) 
        && (find_queue_families(instance, surface, khr_extension, device).is_complete())
        && extensions_supported
        && swapchain_adequate;

    if selection_criteria {
        println!("Selected physical device: {}", device_name.to_str().expect("Failed to convert CStr to string!"));
    }

    selection_criteria
}

unsafe fn check_device_extension_support(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> bool {
    // Not all graphics cards are capable of presenting images directly to a screen.
    // In order to get support for presenting images to the screen, we need to enable the VK_KHR_swapchain extension.
    // This extension indicates whether the device is capable of creating a swapchain.
    // So, we need to query our device for support for this extension.
    let available_device_extensions = instance.enumerate_device_extension_properties(physical_device).unwrap();

    let mut required_extensions = REQUIRED_EXTENSIONS.clone();

    for available_extension in available_device_extensions {
        let extension_name = CStr::from_ptr(available_extension.extension_name.as_ptr()).to_str().unwrap();
        required_extensions.remove(extension_name);
    }

    required_extensions.is_empty()
}

unsafe fn build_extensions() -> Vec<String> {
    let mut required_extensions: Vec<String> = vec!();

    // Get required GLFW extensions
    // GLFW will include VK_KHR_Surface. This is the Window System Integration (WSI) extension. It can be used
    // To establish a connection between Vulkan and the window system.
    // Vulkan is a platform agnostic API, so the core specification has no knowledge of concrete windowing systems.
    let mut glfw_extension_count: u32 = 0;
    let mut glfw_extensions = glfwGetRequiredInstanceExtensions(&mut glfw_extension_count);

    for n in 1..=glfw_extension_count {
        let current_string = *glfw_extensions;
        required_extensions.push(
            String::from_utf8_lossy(CStr::from_ptr(current_string).to_bytes()).to_string());
        glfw_extensions = glfw_extensions.offset(n as isize);
    }

    // VK_EXT_debug_utils is a required extension when setting up callback functionality
    required_extensions.push(String::from("VK_EXT_debug_utils"));

    required_extensions
}

unsafe fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        p_user_data: ptr::null_mut(),
        ..Default::default()
    }
}

unsafe fn setup_debug_messenger(debug_utils_ext: &ash::extensions::ext::DebugUtils) -> vk::DebugUtilsMessengerEXT {
    // Fill out the struct describing the kind of debug messenger we'd like
    let messenger_create_into = populate_debug_messenger_create_info();

    let debug_utils_messenger = debug_utils_ext
        .create_debug_utils_messenger(&messenger_create_into, None)
        .expect("Failed to create Debug Utils Messenger");

    debug_utils_messenger
}

/*
    When communicating with unsafe bindings, I make use of the "CString" type: https://docs.rs/rustc-std-workspace-std/1.0.1/std/ffi/struct.CString.html
    This type represents an owned, C-comptable, null-terminated string.
    The important part for me right now being that it's nul-terminated, which many C APIs expect.
*/
fn ffi_string(str: &str) -> CString {
    let error_message = format!("Failed to generate CString from {}", str);
    CString::new(str).expect(&error_message)
}

unsafe fn get_latest_glfw_error_description() -> String {
    let mut error_description_raw: *const i8 = ptr::null_mut();
    glfwGetError(&mut error_description_raw);
    let error_description = CString::from_raw(error_description_raw as *mut i8);
    error_description.into_string().expect("Failed to convert GLFW error description into String type")
}

// Callback function used by Debug Utils extension.
// TODO: What does extern "system" mean?
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    p_user_data: *mut c_void) -> vk::Bool32 {

        let severity = match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[ERROR]",
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[INFO]",
            _ => "[Unknown]"
        };

        let types = match message_type {
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
            vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
            _ => "[Unknown]"
        };

        let message = CStr::from_ptr((*p_callback_data).p_message);

        println!("[Debug]{}{}{:?}", severity, types, message);

        // The callback returns a boolean that indicates if the Vulkan call that triggered the validation layer message should
        // be aborted. If the callback returns true, the call is aborted.
        // This is normally used used to test the validation layers themselves, so you should always return VK_FALSE.
        vk::FALSE
}