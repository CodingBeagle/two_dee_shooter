use std::ffi::{ CString, CStr };
use std::ptr;
use std::os::raw::c_char;

use ash::{vk, Entry};

use beagle_glfw::*;

static WIDTH: i32 = 800;
static HEIGHT: i32 = 600;

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

            To create an instance, you first hav eto fill out a struct with information about the application.
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
        let mut glfw_extension_count: u32 = 0;
        let glfw_extensions = glfwGetRequiredInstanceExtensions(&mut glfw_extension_count);

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

        let create_info = vk::InstanceCreateInfo {
            s_type: vk::StructureType::INSTANCE_CREATE_INFO,
            p_application_info: &application_info,
            enabled_extension_count: glfw_extension_count,
            pp_enabled_extension_names: glfw_extensions,
            pp_enabled_layer_names: validation_layers_as_raw_pointers.as_ptr(),
            enabled_layer_count: required_validation_layers.len() as u32,
            ..Default::default()
        };

        // Now everything is specified for Vulkan to create an instance
        // This instance should live for as long as the application lives.
        let vk_instance = entry.create_instance(&create_info, None).expect("Failed to create Vulkan instance.");

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

        // Before we terminate the application, we destroy the Vulkan instance.
        vk_instance.destroy_instance(None);

        glfwDestroyWindow(main_window);

        // Before terminating your application, you should terminate the GLFW library if it has been initialized.
        // If you don't global system settings changed by GLFW might not be restored properly.
        glfwTerminate();
    }
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