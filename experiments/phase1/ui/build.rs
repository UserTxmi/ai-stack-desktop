fn main() {
    // Recurso ICO mínimo del ensayo (DIB BGRA, 1x1); sin descargar un icono.
    let dir = std::path::Path::new("../.lab/ui-assets");
    std::fs::create_dir_all(dir).unwrap();
    let mut ico = vec![0,0,1,0,1,0, 1,1,0,0,1,0,32,0,48,0,0,0,22,0,0,0];
    ico.extend_from_slice(&[40,0,0,0,1,0,0,0,2,0,0,0,1,0,32,0]);
    ico.extend_from_slice(&[0; 24]);
    ico.extend_from_slice(&[0x70,0x55,0x11,0xff,0,0,0,0]);
    std::fs::write(dir.join("lab.ico"), ico).unwrap();
    tauri_build::build()
}
