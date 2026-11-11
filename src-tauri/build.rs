fn main() {
    // Tauri embeds a Common Controls v6 manifest into the app binary only. Test binaries link the same
    // UI code and fail to start without it (STATUS_ENTRYPOINT_NOT_FOUND), so we embed one manifest for
    // every target ourselves.
    let mut attrs = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        attrs =
            attrs.windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed=windows-app-manifest.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    // Version of the bundled advisor data: commit time of the newest commit touching data/, the same
    // number meta-pipeline/build-dataset.mjs gives the downloadable dataset. 0 if git isn't available.
    println!("cargo:rerun-if-changed=../data");
    let v = std::process::Command::new("git")
        .args(["log", "-1", "--format=%ct", "--", "data"])
        .current_dir("..")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(0);
    println!("cargo:rustc-env=POSTPLANT_DATA_VERSION={v}");
    tauri_build::try_build(attrs).expect("tauri build");
}
