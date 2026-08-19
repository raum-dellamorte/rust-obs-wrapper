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
    // Using cargo zigbuild to restrict glibc to a version compatible with the flatpak
    // release of OBS breaks the simde dependency, so we're symlinking the simde folder
    // into an include dir local to the install. The zigbuild is meant to be portable,
    // because building on Arch with the latest versions of things does not make for a
    // portable library.
    println!("cargo:rerun-if-env-changed=SIMDE_INCLUDE_DIR");
    println!("cargo:rerun-if-changed=/usr/include/simde");
    clang_args.push("-I/usr/include/obs".to_string());
    let simde_include_root = simde_include_create()
        .unwrap_or_else(|e| panic!("Failed to prepare SIMDe headers: {}", e));
    clang_args.push(format!("-I{}", simde_include_root.display()));
    clang_args.push("-DSIMDE_NO_NATIVE".to_string());
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

#[cfg(target_family = "unix")]
fn simde_include_create() -> Result<PathBuf, Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;
    let simde_source = std::env::var_os("SIMDE_INCLUDE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/include/simde"));
    if !simde_source.join("x86/sse2.h").is_file() {
        return Err(format!(
            "SIMDe headers not found under {}",
            simde_source.display()
        )
        .into());
    }
    let include_root =
        PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is not set")?)
            .join("include");
    std::fs::create_dir_all(&include_root)?;
    let simde_link = include_root.join("simde");
    // symlink_metadata detects dangling symlinks too.
    if std::fs::symlink_metadata(&simde_link).is_ok() {
        std::fs::remove_file(&simde_link)?;
    }
    symlink(&simde_source, &simde_link)?;
    Ok(include_root)
}
