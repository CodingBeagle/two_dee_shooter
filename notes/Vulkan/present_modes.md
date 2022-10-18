# Present Modes

The present mode in Vulkan determines the conditions for showing images to the screen.

There are four possible modes in Vulkan:

- VK_PRESENT_MODE_IMMEDIATE_KHR
  - Images submitted by your application are transferred to the screen right away, which may result in tearing.
- VK_PRESENT_MODE_FIFO_KHR
  - The swap chain is a queue where the display takes an image from the front of the queue when the display is refreshed and the program inserts rendered images at the back of the queue.
  - If the queue is full then the program has to wait.
  - This is most similar to vertical sync in modern games.
- VK_PRESENT_MODE_FIFO_RELAXED_KHR
  - The mode only differs from the previous if the application is late and the queue was empty at the last vertical blank. Instead of waiting for the next vertical blank, the image is transferred right away when it finally arrives. This may result in visible tearing.
- VK_PRESENT_MODE_MAILBOX_KHR
  - Instead of blocking the application when the queue is full, the images that are already queued are simply replaced with the newer ones.
  - This made can be used to render frames as fast as possible while still avoiding tearing, resulting in fewer latency issues than standard vertical sync.
  - This is commonly known as "triple buffering", although the existance of three buffers alone does not necessarily mean that the framerate is unlocked.