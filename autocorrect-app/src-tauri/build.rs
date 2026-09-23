use std::process::Command;

fn main() {
    tauri_build::build();

    #[cfg(target_os = "macos")]
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let swift_dir = std::path::Path::new(&manifest_dir).join("apple-translation");

        let status = Command::new("swift")
            .args(["build", "-c", "release"])
            .current_dir(&swift_dir)
            .status()
            .expect("failed to build Apple Translation Swift package");
        if !status.success() {
            panic!("Apple Translation Swift package build failed");
        }

        let bin_path = Command::new("swift")
            .args(["build", "-c", "release", "--show-bin-path"])
            .current_dir(&swift_dir)
            .output()
            .expect("failed to get Swift bin path");
        let bin_path = String::from_utf8(bin_path.stdout)
            .unwrap()
            .trim()
            .to_string();

        println!("cargo:rustc-link-search=native={}", bin_path);
        println!("cargo:rustc-link-lib=static=AppleTranslation");
        println!("cargo:rustc-link-lib=framework=Translation");
        println!("cargo:rustc-link-search=/usr/lib/swift");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!("cargo:rustc-link-lib=dylib=swiftCore");
        println!("cargo:rustc-link-lib=dylib=swiftFoundation");
        println!("cargo:rustc-link-lib=dylib=swiftDispatch");
        println!("cargo:rustc-link-lib=dylib=swiftObjectiveC");

        println!("cargo:rerun-if-changed=apple-translation/");

        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }
}
