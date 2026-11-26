use super::*;
use tauri::{AppHandle, LogicalPosition, LogicalSize, WebviewBuilder, WebviewUrl};
use windows::Win32::{System::Com::*, UI::Shell::*};

#[tauri::command]
pub async fn elegir() -> Result<Option<String>, String> {
    // STA privado: no cambiar el apartamento COM del hilo de WebView2.
    std::thread::spawn(|| unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|_| "No se pudo abrir el selector")?;
        let result = (|| -> windows::core::Result<Option<String>> {
            let dialog: IFileOpenDialog =
                CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
            dialog.SetOptions(
                dialog.GetOptions()? | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_DONTADDTORECENT,
            )?;
            if let Err(e) = dialog.Show(None) {
                if e.code().0 == 0x800704c7u32 as i32 {
                    return Ok(None);
                }
                return Err(e);
            }
            let name = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
            let path = name.to_string();
            CoTaskMemFree(Some(name.0.cast()));
            Ok(Some(path?))
        })();
        CoUninitialize();
        result.map_err(|_| "No se pudo leer la carpeta seleccionada".to_string())
    })
    .join()
    .map_err(|_| "Selector interrumpido".to_string())?
}
const TOP: f64 = 54.;
pub fn resize(app: &AppHandle) -> tauri::Result<()> {
    if let (Some(window), Some(web)) = (app.get_window("main"), app.get_webview("agent")) {
        let size = window
            .inner_size()?
            .to_logical::<f64>(window.scale_factor()?);
        web.set_position(LogicalPosition::new(0., TOP))?;
        web.set_size(LogicalSize::new(size.width, (size.height - TOP).max(1.)))?;
    }
    Ok(())
}
#[tauri::command]
pub async fn abrir(
    app: AppHandle,
    state: State<'_, Shared>,
    carpeta: String,
) -> Result<(), String> {
    if let Some(web) = app.get_webview("agent") {
        web.close().map_err(|e| e.to_string())?;
    }
    let (url, origin) = access(state, move |_, s| {
        let w = s.open_workspace(std::path::Path::new(&carpeta))?;
        Ok((w.launch.clone(), w.origin.clone()))
    })
    .await?;
    let win = app.get_window("main").ok_or("Ventana principal ausente")?;
    let builder = WebviewBuilder::new(
        "agent",
        WebviewUrl::External(url.parse().map_err(|_| "Origen inválido")?),
    )
    .incognito(true)
    .disable_drag_drop_handler()
    .on_navigation(move |u| u.origin().ascii_serialization() == origin)
    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
    .on_download(|_, _| false);
    win.add_child(
        builder,
        LogicalPosition::new(0., TOP),
        LogicalSize::new(640., 420.),
    )
    .map_err(|e| e.to_string())?;
    resize(&app).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn vista(app: AppHandle, visible: bool) -> Result<(), String> {
    if let Some(web) = app.get_webview("agent") {
        if visible { web.show() } else { web.hide() }.map_err(|e| e.to_string())?;
    }
    Ok(())
}
