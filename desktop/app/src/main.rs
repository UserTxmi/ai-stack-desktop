#![windows_subsystem = "windows"]
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};
use w1_supervisor::{Status, Supervisor};
type Shared = Arc<Mutex<Supervisor>>;

async fn access<T: Send + 'static>(
    state: State<'_, Shared>,
    op: impl FnOnce(&mut Supervisor) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        op(&mut *shared.lock().map_err(|_| "Supervisor no disponible")?)
    })
    .await
    .map_err(|_| "Tarea interrumpida".to_string())?
}
#[tauri::command]
async fn estado(state: State<'_, Shared>) -> Result<Status, String> {
    access(state, |s| Ok(s.status())).await
}
#[tauri::command]
async fn iniciar(state: State<'_, Shared>) -> Result<(), String> {
    access(state, Supervisor::start).await
}
#[tauri::command]
async fn detener(state: State<'_, Shared>) -> Result<(), String> {
    access(state, Supervisor::stop).await
}
#[tauri::command]
async fn probar(state: State<'_, Shared>) -> Result<(), String> {
    access(state, Supervisor::demo_turn).await
}
fn main() {
    w1_supervisor::worker_mode();
    let here = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let runtime = here.join("experiments/phase1/.lab/runtime");
    let base =
        PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("Se necesita perfil de usuario"))
            .join("AI Stack Desktop/Demo W1");
    let supervisor = Supervisor::new(runtime, base);
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--prueba-backend")) {
        let mut s = supervisor;
        let outcome = s.start().and_then(|_| s.demo_turn());
        let running = s.status();
        let close = s.stop();
        let passed = outcome.is_ok()
            && close.is_ok()
            && running
                .services
                .iter()
                .all(|r| r.active && r.healthy && r.metrics.is_some());
        let mut report = format!("aceptada={passed}\n");
        for row in running.services {
            report += &format!(
                "{} activa={} sana={} pico_job_bytes_y_cpu_s={:?}\n",
                row.name, row.active, row.healthy, row.metrics
            );
        }
        for event in s.status().events {
            report += &format!("{event}\n");
        }
        if let Some(e) = outcome.err().or(close.err()) {
            report += &format!("Error: {e}\n");
        }
        std::fs::write(here.join("backend.txt"), report).unwrap();
        std::process::exit(if passed { 0 } else { 1 });
    }
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(supervisor)))
        .invoke_handler(tauri::generate_handler![estado, iniciar, detener, probar])
        .build(tauri::generate_context!())
        .expect("No se pudo abrir WebView2")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                if let Ok(mut s) = app.state::<Shared>().lock() {
                    let _ = s.stop();
                }
            }
        });
}
