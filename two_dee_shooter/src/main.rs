use std::borrow::Borrow;
use std::ffi::{ CString, CStr, c_void };
use std::ptr;
use std::os::raw::c_char;

use ash::{vk, Entry};

use beagle_glfw::*;

static WIDTH: i32 = 800;
static HEIGHT: i32 = 600;

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

fn main() {
    unsafe {
        if glfwInit() == 0 {
            panic!("Failed to initialize GLFW.");
        }

        // Vulkan Ash related initialization
        // TODO: Read up more on this Entry::Linked called. It seems to load the Vulkan library by linking to it statically.
        // But how does this work, and what exactly does it do???
        let entry = Entry::linked();

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
        let available_layers = entry.enumerate_instance_layer_properties().expect("Failed to retrieve available layers.");

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
        let vk_instance = entry.create_instance(&create_info, None).expect("Failed to create Vulkan instance.");

        // In order to create a debug messenger, we have to call the function "vkCreateDebugUtilsMessengerEXT"
        // Since this is an extension function, it is not automatically loaded with Vulkan.
        // We have to load it ourselves
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &vk_instance);
        let debug_utils_messenger = setup_debug_messenger(&debug_utils_loader);

        // After creating a Vulkan instance, we need to select a physical graphics card that supports the features we need.
        let physical_devices = vk_instance.enumerate_physical_devices().expect("Failed to retrieve physical devices.");

        let mut selected_physical_device: Option<vk::PhysicalDevice> = None;
        for physical_device in physical_devices {
            if is_device_suitable(&vk_instance, physical_device) {
                selected_physical_device = Some(physical_device);
            }
        }

        if selected_physical_device.is_none() {
            panic!("Failed to select a physical device!");
        }

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

        while glfwWindowShouldClose(main_window) == 0 {
            glfwPollEvents();
        }

        // Clean up the debug messenger
        // Destroying the debug messenger must be done before the Vulkan instance is destroyed.
        // TODO: Does Ash handle any of these calls in Drop implementations of the structs??
        debug_utils_loader.destroy_debug_utils_messenger(debug_utils_messenger, None);

        // Before we terminate the application, we destroy the Vulkan instance.
        vk_instance.destroy_instance(None);

        glfwDestroyWindow(main_window);

        // Before terminating your application, you should terminate the GLFW library if it has been initialized.
        // If you don't global system settings changed by GLFW might not be restored properly.
        glfwTerminate();
    }
}

unsafe fn is_device_suitable(instance: &ash::Instance, device: vk::PhysicalDevice) -> bool {
    let device_properties = instance.get_physical_device_properties(device);
    let device_features = instance.get_physical_device_features(device);

    let device_name = CStr::from_ptr(device_properties.device_name.as_ptr());
    println!("Checking physical device: {}", device_name.to_str().expect("Failed to convert CStr to string!"));

    // Currently, I just select any physical GPU that supports geometry shaders.
    return device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU && device_features.geometry_shader > 0
}

unsafe fn build_extensions() -> Vec<String> {
    let mut required_extensions: Vec<String> = vec!();

    // Get required GLFW extensions
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