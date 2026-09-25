use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(pvr_compiler_available)");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    let root = Path::new("vendor");
    let mut build = cc::Build::new();
    build
        .warnings(true)
        .extra_warnings(false)
        .flag("-Wno-unused")
        // Preserve upstream optimization decisions for driver diagnostics. These
        // legacy warnings describe that code, rather than the host adapter.
        .flag("-Wno-switch")
        .flag("-Wno-enum-compare")
        .flag("-Wno-bool-compare")
        .flag("-Wno-logical-not-parentheses")
        .flag("-Wno-tautological-compare")
        .flag("-Wno-address")
        .flag("-Wno-misleading-indentation")
        .flag("-Wno-sizeof-pointer-memaccess")
        .flag("-Wno-maybe-uninitialized")
        // USC reinterprets float/integer bits through pointers throughout.
        // Modern GCC's type-based alias analysis is invalid for that code.
        .flag("-fno-strict-aliasing")
        .std("gnu99")
        .flag("-fcommon")
        .flag("-Werror")
        .flag("-include")
        .flag("host.h")
        .define("NDEBUG", None)
        .define("USER", None)
        .define("LINUX", None)
        .define("STANDALONE", None)
        .define("GLSL_ES", None)
        .define("GEN_HW_CODE", None)
        .define("OUTPUT_USPBIN", None)
        .define("SUPPORT_SGX543", None)
        .define("SUPPORT_SGX", None)
        .define("INCLUDE_SGX_FEATURE_TABLE", Some(""))
        .define("INCLUDE_SGX_BUG_TABLE", Some(""));
    for dir in [
        "tools/intern/oglcompiler/glsl",
        "tools/intern/oglcompiler/parser",
        "tools/intern/oglcompiler/powervr",
        "tools/intern/oglcompiler/binshader",
        "tools/intern/usc2",
        "tools/intern/useasm",
        "tools/intern/usp",
        "include/gpu_es4/eurasia/include4",
        "include/gpu_es4/eurasia/hwdefs",
        "include/gpu_es4/eurasia/services4/include",
        "include/gpu_es4/eurasia/services4/srvclient/devices/sgx",
        "eurasiacon/include",
        "eurasiacon/common",
        "intermediates/glslparser",
        "intermediates/sgxsupport",
        "intermediates/errata",
        "codegen/pixfmts",
        "eurasiacon/opengles2",
    ] {
        build.include(root.join(dir));
    }
    let mut sources = Vec::new();
    for dir in [
        "tools/intern/oglcompiler/glsl",
        "tools/intern/oglcompiler/parser",
        "tools/intern/oglcompiler/powervr",
        "tools/intern/usc2",
        "tools/intern/useasm",
        "intermediates/glslparser",
    ] {
        for file in fs::read_dir(root.join(dir)).expect("read vendored compiler sources") {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "c") {
                sources.push(path);
            }
        }
    }
    sources.sort();
    build
        .files(sources)
        .file(root.join("tools/intern/oglcompiler/binshader/esbinshader.c"))
        .file("bridge.c")
        .compile("pvr_compiler");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-cfg=pvr_compiler_available");
    for path in ["vendor", "bridge.c", "host.h"] {
        println!("cargo:rerun-if-changed={path}");
    }
}
