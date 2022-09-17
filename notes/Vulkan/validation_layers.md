# Validation Layers

The Vulkan API is designed around the idea of minimal driver overhead.

As such, there is very limited error checking in the API by default. Even things such as checking that a required parameter isn't null, or passing incorrect enumeration values, won't generally be checked. These errors will simply result in crashes or undefined behaviour.

But, the Vulkan API is very explicit. As such, it's easy to make many small mistakes.

Validation Layers are a feature of the API that allows you to add optional components that hook into Vulkan function calls to produce additional operations.

Common validation layer operations could be:

- Checking the values of parameters against the specification to detect misuse.
- Tracking creation and destruction of objects to find resource leaks.
- Checking thread safety by tracking the threads that calls originate from.
- Logging every call and its parameters to standard output.
- Tracing Vulkan calls for profiling and replaying.

