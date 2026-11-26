#![windows_subsystem = "windows"]
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, WebviewWindowBuilder};
use w1_supervisor::{Install, Progress, Status, Supervisor};

#[derive(Clone)]
struct Shared {
    install: Arc<Install>,
    supervisor: Arc<Mutex<Supervisor>>,
    // La UI consulta el progreso sin esperar al supervisor mientras descarga/prepara.
    progress: Arc<Mutex<Option<Progress>>>,
}

async fn access<T: Send + 'static>(
    state: State<'_, Shared>,
    op: impl FnOnce(Shared, &mut Supervisor) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut supervisor = shared
            .supervisor
            .lock()
            .map_err(|_| "Supervisor no disponible")?;
        op(shared.clone(), &mut supervisor)
    })
    .await
    .map_err(|_| "Tarea interrumpida".to_string())?
}

#[tauri::command]
fn ubicacion(state: State<'_, Shared>) -> String {
    state.install.root().to_string_lossy().into_owned()
}
#[tauri::command]
fn progreso(state: State<'_, Shared>) -> Option<Progress> {
    state.progress.lock().ok()?.clone()
}
#[tauri::command]
async fn estado(state: State<'_, Shared>) -> Result<Status, String> {
    access(state, |_, s| Ok(s.status())).await
}
#[tauri::command]
async fn preparar(state: State<'_, Shared>) -> Result<(), String> {
    access(state, |share, s| {
        if s.running() {
            return Err("Detenga el stack antes de preparar los runtimes".into());
        }
        let result = w1_supervisor::prepare(&share.install, |p| {
            *share.progress.lock().unwrap() = Some(p)
        })
        .map(|_| ());
        *share.progress.lock().unwrap() = None;
        s.set_preparation(result.clone());
        result
    })
    .await
}
#[tauri::command]
async fn iniciar(state: State<'_, Shared>) -> Result<(), String> {
    access(state, |_, s| s.start()).await
}
#[tauri::command]
async fn detener(state: State<'_, Shared>) -> Result<(), String> {
    access(state, |_, s| s.stop()).await
}
#[tauri::command]
async fn probar(state: State<'_, Shared>) -> Result<(), String> {
    access(state, |_, s| s.demo_turn()).await
}

fn alert(msg: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR};
    let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(msg).as_ptr(),
            wide("AI Stack Desktop").as_ptr(),
            MB_ICONERROR,
        );
    }
}

// Modos headless: el mismo backend de la interfaz; ninguna entrada de instalación desde IPC.
fn ci(args: &[std::ffi::OsString]) -> Option<Result<(), String>> {
    let mode = args.first()?.to_str()?;
    if !matches!(
        mode,
        "--ci-preparar" | "--ci-verificar" | "--prueba-backend" | "--ci-cierre-forzado"
    ) {
        return None;
    }
    let run = || {
        let report = args.get(1).ok_or("Falta archivo de informe")?;
        let result = Install::beside_exe().and_then(|install| {
            if mode == "--ci-verificar" {
                w1_supervisor::verify(&install)?;
                return Ok("W1 verificado\n".to_string());
            }
            if mode == "--ci-preparar" {
                let stats = w1_supervisor::prepare(&install, |_| {})?;
                return Ok(format!(
                    "W1 verificado; reutilizados={}; instalados={}; descargas={}\n",
                    stats.reused, stats.installed, stats.downloads
                ));
            }
            let mut supervisor = Supervisor::new(Arc::new(install));
            if mode == "--ci-cierre-forzado" {
                supervisor.start()?;
                let pids = supervisor.owned_pids()?;
                std::fs::write(
                    report,
                    pids.iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                )
                .map_err(|e| e.to_string())?;
                std::thread::sleep(std::time::Duration::from_secs(120));
                return Err("El ensayo no terminó el propietario dentro del plazo".into());
            }
            let (text, passed) = backend(supervisor);
            if passed {
                Ok(text)
            } else {
                Err(text)
            }
        });
        let text = match &result {
            Ok(s) | Err(s) => s,
        };
        std::fs::write(report, text).map_err(|e| format!("Escribir informe: {e}"))?;
        result.map(|_| ())
    };
    Some(run())
}

fn backend(mut s: Supervisor) -> (String, bool) {
    let outcome = s.start().and_then(|_| s.demo_turn());
    let running = s.status();
    let close = s.stop();
    let stopped = s.status();
    let passed = outcome.is_ok()
        && close.is_ok()
        && running
            .services
            .iter()
            .all(|r| r.active && r.healthy && r.metrics.is_some())
        && stopped.services.iter().all(|r| !r.active);
    let mut report = format!("aceptada={passed}\n");
    for row in running.services {
        report += &format!(
            "{} activa={} sana={} pico_job_bytes_y_cpu_s={:?}\n",
            row.name, row.active, row.healthy, row.metrics
        );
    }
    for event in stopped.events {
        report += &format!("{event}\n");
    }
    if let Some(e) = outcome.err().or(close.err()) {
        report += &format!("Error: {e}\n");
    }
    (report, passed)
}

fn main() {
    w1_supervisor::worker_mode();
    if let Some(result) = ci(&std::env::args_os().skip(1).collect::<Vec<_>>()) {
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    let install = match Install::beside_exe() {
        Ok(install) => Arc::new(install),
        Err(e) => {
            alert(&e);
            return;
        }
    };
    let webview = match install.webview() {
        Ok(path) => path,
        Err(e) => {
            alert(&e);
            return;
        }
    };
    // WebView2 admite este ajuste por proceso, también si existía en el entorno heredado.
    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &webview);
    let shared = Shared {
        supervisor: Arc::new(Mutex::new(Supervisor::new(install.clone()))),
        progress: Arc::default(),
        install,
    };
    let check = shared.clone();
    match tauri::Builder::default()
        .manage(shared)
        .setup(move |app| {
            let config = &app.config().app.windows[0];
            WebviewWindowBuilder::from_config(app, config)?.data_directory(webview).build()?;
            tauri::async_runtime::spawn_blocking(move || {
                if let Ok(mut s) = check.supervisor.lock() { s.set_preparation(w1_supervisor::verify(&check.install)); }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ubicacion, progreso, estado, preparar, iniciar, detener, probar])
        .build(tauri::generate_context!())
    {
        Ok(app) => app.run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                // Si está preparando, no esperar procesos externos al cerrar. Los Jobs
                // cierran al terminar el EXE y la siguiente preparación limpia el staging.
                if let Ok(mut s) = app.state::<Shared>().supervisor.try_lock() { let _ = s.stop(); }
            }
        }),
        Err(e) => alert(&format!("No se pudo abrir la ventana. Compruebe WebView2 Runtime y que la carpeta sea escribible.\n\n{e}")),
    }
}
