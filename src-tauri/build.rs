fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "greet",
                "add_host",
                "get_hosts",
                "delete_host",
                "add_group",
                "get_groups",
                "open_ssh_session",
                "write_to_ssh",
            ]),
        ),
    )
    .expect("failed to run tauri-build");
}
