extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

#[cfg(windows)]
mod build_win;

#[cfg(target_os = "macos")]
mod build_mac;

fn main() {
  // Tell cargo to invalidate the built crate whenever the wrapper changes
  println!("cargo:rerun-if-changed=wrapper.h");
  println!("cargo:rerun-if-env-changed=DONT_USE_GENERATED_BINDINGS");
  println!("cargo:rerun-if-env-changed=TARGET");
  let proj_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  println!("cargo:rustc-link-search=native={}/deps", proj_dir);

  let mut clang_args: Vec<String> = vec![];

  if std::env::var("CARGO_CFG_TARGET_OS").map(|s| s.contains("linux") ).unwrap_or(false) {
    clang_args.push("-I/usr/include/obs".to_string());
  } else if std::env::var("CARGO_CFG_TARGET_OS").map(|s| s.contains("macos") ).unwrap_or(false) {
    #[cfg(target_os = "macos")]
    build_mac::find_mac_obs_lib();
  } else if std::env::var("CARGO_CFG_TARGET_OS").map(|s| s.contains("windows") ).unwrap_or(false) {
    if std::env::var("HOST").map(|s| s.contains("linux") ).unwrap_or(false) {
      clang_args.push("-I/usr/include/obs".to_string());
    } else {
      #[cfg(windows)]
      build_win::find_windows_obs_lib();
    }
    println!("cargo:rustc-link-lib=dylib=obs");
    println!("cargo:rustc-link-lib=dylib=obs-frontend-api");
    clang_args.push("-Wno-error=implicit-function-declaration".into());
    clang_args.push(format!("-I{}/deps", proj_dir));
  }

  let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

  let builder = bindgen::Builder::default()
    .header("wrapper.h")
    .clang_args(clang_args)
    .blocklist_type("_bindgen_ty_2")
    .blocklist_type("_bindgen_ty_3")
    .blocklist_type("_bindgen_ty_4")
    .derive_default(true)
    .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

  match builder.generate() {
    Ok(bindings) => {
      bindings
        .write_to_file(&out_path)
        .expect("Couldn't write bindings!");
      fs::copy(&out_path, "generated/bindings.rs").expect("Could not copy bindings!");
    }

    Err(e) => {
      if env::var("DONT_USE_GENERATED_BINDINGS").is_ok() {
        panic!("Failed to generate headers with bindgen: {}", e);
      }

      println!("cargo:warning=Could not find obs headers - using pre-compiled.");
      println!("cargo:warning=This could result in a library that doesn't work.");
      fs::copy("generated/bindings.rs", out_path).expect("Could not copy bindings!");
    }
  }
}
