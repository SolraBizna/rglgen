This is a binding generator. It can produce an efficient (but unsafe) direct Rust binding for any version of OpenGL and any set of desired extensions. The resultant binding is compatible with any operating system that provides something akin to [`SDL_GL_GetProcAddress`][1] (which should be every operating system that has OpenGL).

# Usage

Install `rglgen` somehow (perhaps with `cargo install rglgen`).

Download [`gl.xml`][2] from the Khronos Group and put it somewhere known.

Run rglgen, tell it what OpenGL version you want, give it the path to `gl.xml`, specify any additional extensions you're interested in, and redirect the output appropriately:

```sh
rglgen ~/nobackup/gl.xml -t gl3.1 GL_ARB_debug_output > src/gl31.rs
```

This example produces a complete binding for OpenGL 3.1, along with the GL_ARB_debug_output extension. It puts the binding into a module named `gl31` in the target directory.

In your program, you must initialize an instance of `Procs`:

```rust
// (This example is for use with the sdl2 crate)
let gl = Procs::new(|proc| {
    // Our SDL2 binding provides a `gl_get_proc_address` binding, but it
    // only takes &str and adds a null terminator to it before calling.
    // rglgen bindings already contain the null terminator. So, we need to
    // call it ourselves.
    //
    // Unsafe justification: the input is known to be a static,
    // null-terminated string.
    let ret = unsafe {
        sdl2_sys::SDL_GL_GetProcAddress(transmute(proc.as_ptr()))
    };
    if ret.is_null() {
        Err(anyhow!("Unable to find the procedure named {}: {}",
                    String::from_utf8_lossy(&proc[..proc.len()-1]),
                    sdl2::get_error()))
    }
    else {
        // Unsafe justification: a non-null return address is a valid
        // OpenGL procedure entry point.
        Ok(unsafe{transmute(ret)})
    }
})?;
```

And now you can call OpenGL routines:

```rust
unsafe {
    gl.ClearColor(0.4, 0.5, 0.6, 1.0);
    gl.Clear(GL_COLOR_BUFFER_BIT);
}
```

You must have a separate `Procs` instance for every OpenGL context you create, which is a bummer. However, this also means that multiple different OpenGL bindings can coexist in the same crate at both compile time and runtime. You can even have different windows open with different OpenGL versions and correctly bind each one.

To save some runtime overhead and compile time, you can make a "used identifiers" file. It's an ordinary text file, containing one line for every identifier (function call or constant) that your program uses. Pass this to `rglgen` with the `-u` option and it will bind only those identifiers. This saves it from having to fetch and store the addresses of procs you never call.

The produced binding will have almost no documentation. I strongly recommend [`docs.gl`][3] for all your OpenGL reference needs.

# Legalese

rglgen is copyright 2022 and 2023, Solra Bizna, and licensed under either of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or
   <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the rglgen crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

Output of rglgen is explicitly exempted from being under any particular license. The license of [`gl.xml`][2] may be relevant.

[1]: https://wiki.libsdl.org/SDL2/SDL_GL_GetProcAddress
[2]: https://raw.githubusercontent.com/KhronosGroup/OpenGL-Registry/main/xml/gl.xml
[3]: https://docs.gl/
