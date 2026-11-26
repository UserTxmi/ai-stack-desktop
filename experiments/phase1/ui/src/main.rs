#![windows_subsystem = "windows"]
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    let port: u16 = std::env::args().nth(1).expect("falta puerto del ensayo").parse().expect("puerto inválido");
    let url: tauri::Url = format!("http://127.0.0.1:{port}/").parse().unwrap();
    tauri::Builder::default()
        .setup(move |app| {
            let origin = url.origin();
            WebviewWindowBuilder::new(app, "workspace", WebviewUrl::External(url))
                .title("Workspace oficial · ensayo A")
                .on_navigation(move |dest| dest.origin() == origin)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!()).expect("error de WebView");
}
