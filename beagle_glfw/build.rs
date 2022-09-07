/*
    Cargo will compile and execute build.rs before building the actual package.
*/
extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to look for GLFW libraries in the specified directory
    println!("cargo:rustc-link-search=D:/libraries/glfw337");

    // Tell cargo to tell rustc to link the GLFW library
    // I'm currently using the dynamic library (.DLL) version.
    // Using this, I'm not required to also handle linking of GLFW's dependencies, which are:
    // gdi32, user32, and kernel32.
    // More info here: https://www.glfw.org/docs/latest/build_guide.html
    println!("cargo:rustc-link-lib=dylib=glfw3dll");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // bindgen::Builder is the main entry point to bindgen, and lets you build up options for the resulting bindings.
    let bindings = bindgen::Builder::default()
        // I'm currently using the dynamic library (.dll) for GLFW
        // When using that, it is required to define the GLFW_DLL macro, to tell the compiler that the GLFW functions are defined in a DLL.
        .clang_arg("-D GLFW_DLL")
        // The wrapper header contains the header files from GLFW that the generated bindings should be based upon.
        .header("wrapper.h")
        // With the F argument, I add the directory to search for header files for GLFW
        .clang_arg("-FD:/libraries/glfw337")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings.");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    // The OUT_DIR is an environment variable for the build process, and is the folder inside the build directory for the package being built,
    // and it is unique for the package in question.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}