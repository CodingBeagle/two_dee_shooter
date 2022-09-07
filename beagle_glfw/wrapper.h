// We need to define GLFW_INCLUDE_VULKAN in order to indicate to GLFW that we want Vulkan support.
// Defining, or including, the Vulkan header before including GLFW, means that GLFW functions that take or return Vulkan types will be declared.
#define GLFW_INCLUDE_VULKAN
#include <glfw3.h>
#include <glfw3native.h>