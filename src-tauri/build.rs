extern crate bindgen;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    tauri_build::build();

    // env_logger::Builder::new()
    //     .filter(None, LevelFilter::Info)
    //     .init();

    let dir = env::current_dir().unwrap();
    let lib_dir = dir.join("lib");
    //eprintln!("{}", lib_dir.to_string());

    println!("cargo:rustc-link-search=all={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=vvenc");
    //println!("cargo:rustc-link-lib=static=vvenc");

    if env::var("PROFILE").unwrap() == "release" {
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
            // rename_files(&dir, "dylib").unwrap();
        } else if cfg!(target_os = "linux") {
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/vvconvert:{}",
                lib_dir.display()
            );
            //copy_shared_lib(&dir, &target_path, "so").unwrap();

            // rename_files(&dir, "so").unwrap();
        }
    }

    if env::var("PROFILE").unwrap() == "debug" {
        //let target = env::var("TARGET").unwrap();
        //eprintln!("Target (from env var): {}", target);

        if cfg!(target_os = "windows") {
            create_symlinks_for_dlls();
            //println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
            //copy_shared_lib(&dir, &target_path, "dll").unwrap();
            // rename_files(&dir, "dll").unwrap();
        } else if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
            //copy_shared_lib(&dir, &target_path, "dylib").unwrap();
            // rename_files(&dir, "dylib").unwrap();
        } else if cfg!(target_os = "linux") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
            //copy_shared_lib(&dir, &target_path, "so").unwrap();

            // rename_files(&dir, "so").unwrap();
        }
    }

    let bindings = bindgen::Builder::default()
        .header("lib/vvenc/vvenc.h")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // need to add gitignore
    fn create_symlinks_for_dlls() {
        let dir = std::env::current_dir().unwrap();
        let lib_dir = dir.join("lib");
        let target_path = dir.join("target/debug");

        if let Ok(entries) = fs::read_dir(&lib_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension() == Some(OsStr::new("dll")) {
                        let link_path = target_path.join(path.file_name().unwrap());
                        if link_path.exists() {
                            fs::remove_file(&link_path).expect("Failed to remove existing symlink");
                        }
                        fs::hard_link(&path, &link_path).expect("Failed to create symlink");
                    }
                }
            }
        }
    }
}
