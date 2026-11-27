#![windows_subsystem = "windows"]
fn main() {
    // Gentle AI invoca `pi install` como un programa Win32. Pasar argv a Node
    // directamente evita el segundo parseo por cmd.exe de rutas con espacios.
    let exe = std::env::current_exe().unwrap();
    if let Some(name @ ("pi.exe" | "setup-companion.exe")) =
        exe.file_name().and_then(|s| s.to_str())
    {
        use std::os::windows::process::CommandExt;
        let runtime =
            std::path::PathBuf::from(std::env::var_os("AI06_RUNTIME").expect("runtime privado"));
        let mut cmd = if name == "pi.exe" {
            let mut c =
                std::process::Command::new(runtime.join("node/node-v24.21.0-win-x64/node.exe"));
            c.arg(
                runtime
                    .join("gentle/node_modules/@earendil-works/pi-coding-agent/dist/bundle/cli.js"),
            );
            c
        } else {
            let mut c = std::process::Command::new(
                runtime.join("gentle/node_modules/gentle-pi/.gentle-ai/v5.0.0/gentle-ai.exe"),
            );
            let path =
                std::env::join_paths(std::iter::once(exe.parent().unwrap().to_owned()).chain(
                    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
                ))
                .unwrap();
            c.env("PATH", path);
            c
        };
        std::process::exit(
            cmd.args(std::env::args_os().skip(1))
                .creation_flags(0x08000000)
                .status()
                .ok()
                .and_then(|s| s.code())
                .unwrap_or(1),
        );
    }
    let prompt = match std::env::args().nth(1).as_deref() {
        Some("--ci-cuentas") => Some("GS-INTEGRADO".to_string()),
        None => None,
        _ => std::process::exit(2),
    };
    let result = w1_supervisor::Install::beside_exe()
        .and_then(|install| w1_supervisor::run_gentle(std::sync::Arc::new(install), prompt));
    if let Err(e) = result {
        eprintln!("AI Stack Desktop 0.0.7-dev: {e}");
        if std::env::args().len() == 1 {
            let msg: Vec<u16> = e.encode_utf16().chain([0]).collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    windows_sys::w!("AI Stack Desktop 0.0.7-dev"),
                    16,
                );
            }
        }
        std::process::exit(1);
    }
}
