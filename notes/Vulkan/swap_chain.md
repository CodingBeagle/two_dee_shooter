# Swap Chains

Vulkan does not have the concept of a default framebuffer.

It instead requires an infrastructure that will own the buffers that we render to, before visualizing them on the screen.