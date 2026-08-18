fn main() {
    println!("cargo:rustc-link-lib=dylib=comctl32");
    println!("cargo:rerun-if-changed=toolsplus.exe.manifest");

    let manifest_content = include_str!("toolsplus.exe.manifest");

    let windows = tauri_build::WindowsAttributes::new()
        .app_manifest(manifest_content);

    tauri_build::try_build(
        tauri_build::Attributes::new().windows_attributes(windows),
    )
    .expect("failed to run tauri-build");
}
