use std::{env, path::PathBuf};

fn main() {
    if !std::path::Path::new("nuklear/nuklear.h").exists() {
        panic!("nuklear/nuklear.h is missing. Run `git submodule update --init` in the repository root.");
    }
    // The flags remove the undefined behaviour of C signed overflow and aliasing on gcc and clang.
    cc::Build::new()
        .file("src/nuklear.c")
        .warnings(false)
        .flag_if_supported("-fwrapv")
        .flag_if_supported("-fno-strict-aliasing")
        .compile("nuklear");

    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .allowlist_function("nk_.*")
        .allowlist_type("nk_.*")
        .allowlist_var("NK_.*")
        .prepend_enum_name(false)
        .derive_default(true)
        .generate()
        .expect("bindgen failed");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out.join("bindings.rs")).expect("cannot write bindings.rs");

    for f in ["src/nuklear.c", "src/config.h", "src/wrapper.h", "nuklear/nuklear.h"] {
        println!("cargo:rerun-if-changed={f}");
    }
}
