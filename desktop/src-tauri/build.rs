fn main() {
    if std::env::var("PROFILE").as_deref() != Ok("release") {
        stage_check_sidecar();
    }
    tauri_build::build()
}

fn stage_check_sidecar() {
    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return;
    };
    let Some(target) = std::env::var_os("TARGET") else {
        return;
    };

    let sidecars_dir = std::path::Path::new(&manifest_dir).join("binaries");
    let sidecar = sidecars_dir.join(format!("chan-{}", target.to_string_lossy()));
    println!("cargo:rerun-if-changed={}", sidecar.display());

    if sidecar.exists() {
        return;
    }

    std::fs::create_dir_all(&sidecars_dir).expect("creating Tauri sidecar dir");
    std::fs::write(&sidecar, b"").expect("creating check-only Tauri sidecar placeholder");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&sidecar, std::fs::Permissions::from_mode(0o755))
            .expect("marking check-only Tauri sidecar placeholder executable");
    }
}
