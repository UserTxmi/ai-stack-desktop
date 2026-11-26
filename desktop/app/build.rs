fn main() {
    // Recurso BGRA propio, también utilizado como icono de ventana por Tauri.
    let mut ico = vec![
        0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0, 48, 0, 0, 0, 22, 0, 0, 0,
    ];
    ico.extend_from_slice(&[40, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 32, 0]);
    ico.extend_from_slice(&[0; 24]);
    ico.extend_from_slice(&[0x89, 0x84, 0x21, 0xff, 0, 0, 0, 0]);
    std::fs::create_dir_all("build").unwrap();
    std::fs::write("build/desktop.ico", ico).unwrap();
    let commands = &[
        "ubicacion",
        "progreso",
        "estado",
        "preparar",
        "iniciar",
        "detener",
        "probar",
        "elegir",
        "abrir",
        "vista",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(commands)),
    )
    .unwrap();
}
