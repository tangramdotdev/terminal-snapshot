use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let crate_src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let crate_out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ghostty_src = crate_src.parent().unwrap().join("ghostty");
    let ghostty_out = crate_out.join("ghostty");

    let profile = env::var("PROFILE").unwrap();
    let mut cmd = Command::new("zig");
    cmd.arg("build")
        .arg("-Demit-lib-vt=true")
        .arg("-Dsimd=false")
        .args(["-p", ghostty_out.to_str().unwrap()]);
    if profile == "release" {
        cmd.arg("-Doptimize=ReleaseFast");
    }
    let output = cmd
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .current_dir(&ghostty_src)
        .spawn()
        .expect("failed to run `zig build`")
        .wait_with_output()
        .expect("failed to wait process");
    if !output.status.success() {
        eprintln!("zig build exited with status: {:?}", output.status);
        std::io::stdout().write_all(&output.stdout).ok();
        std::io::stderr().write_all(&output.stderr).ok();
        std::process::exit(1);
    }

    println!("cargo:rustc-link-search=native={}", ghostty_out.join("lib").display());
    println!("cargo:rustc-link-lib=static=ghostty-vt");
    println!("cargo:rerun-if-changed={}", ghostty_src.join("include").display());

    let include_dir = ghostty_src.join("include");
    let header = include_dir.join("ghostty/vt.h");
    bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.display()))
        .clang_arg("-DGHOSTTY_STATIC")
        .allowlist_function("ghostty_.*")
        .allowlist_type("Ghostty.*")
        .allowlist_var("GHOSTTY_.*")
        .layout_tests(false)
        .generate()
        .expect("failed to generate bindings")
        .write_to_file(crate_out.join("bindings.rs"))
        .expect("failed to write bindings");
}
