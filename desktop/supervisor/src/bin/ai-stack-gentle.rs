#![windows_subsystem = "windows"]
fn main() {
    w1_supervisor::worker_mode();
    let prompt = match std::env::args().nth(1).as_deref() {
        Some("--ci-cuentas") => Some("GS-INTEGRADO".to_string()),
        None => None,
        _ => std::process::exit(2),
    };
    let result = w1_supervisor::Install::beside_exe()
        .and_then(|install| w1_supervisor::run_gentle(std::sync::Arc::new(install), prompt));
    if let Err(e) = result {
        eprintln!("AI Stack Desktop 0.0.6-dev: {e}");
        if std::env::args().len() == 1 {
            let msg: Vec<u16> = e.encode_utf16().chain([0]).collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    windows_sys::w!("AI Stack Desktop 0.0.6-dev"),
                    16,
                );
            }
        }
        std::process::exit(1);
    }
}
