// Bootstrap y diagnóstico por consola; GPUI es la única interfaz de trabajo.
fn main() {
    w1_supervisor::worker_mode();
    let prompt = match std::env::args().nth(1).as_deref() {
        Some("--ci-turno") => Some("GS-INTEGRADO".to_string()),
        Some("--ci-delegar") => Some("LAB-DELEGAR".to_string()),
        None => None,
        _ => std::process::exit(2),
    };
    let result = w1_supervisor::Install::beside_exe()
        .and_then(|install| w1_supervisor::run_gentle(std::sync::Arc::new(install), prompt));
    if let Err(e) = result {
        eprintln!("AI Stack Desktop 0.0.5-dev: {e}");
        if std::env::args().len() == 1 {
            eprintln!("Pulse Entrar para salir.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(1);
    }
}
