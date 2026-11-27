use super::*;
use accounts::{atomic, Accounts};

fn write_changed(p: &Path, bytes: &[u8]) -> Result<()> {
    if install::item(p)?.is_some() && fs::read(p).safe()? == bytes {
        return Ok(());
    }
    atomic(p, bytes)
}

fn chat_default(cfg: &mut Value, models: &[Value]) {
    if models.iter().any(|m| m["id"] == cfg["defaultModel"]) {
        return;
    }
    if let Some(m) = ["cpa/gpt-6.2-sol", "cpa/gpt-6.2-astra"]
        .iter()
        .find_map(|id| models.iter().find(|m| m["id"].as_str() == Some(*id)))
        .or(models.first())
    {
        cfg["defaultModel"] = m["id"].clone();
    } else if let Some(o) = cfg.as_object_mut() {
        o.remove("defaultModel");
    }
}

fn read_json(p: &Path) -> Result<Value> {
    if install::item(p)?.is_none() {
        return Ok(json!({}));
    }
    serde_json::from_slice(&fs::read(p).safe()?).safe()
}
fn setup(home: &Path, runtime: &Path, env: &Env, project: &Path) -> Result<()> {
    let marker = Path::new(&env["GENTLE_SHELL_CONFIG"]);
    let key = s(&dunce::canonicalize(home).safe()?);
    let ready = || -> Result<bool> {
        let doc = read_json(marker)?;
        let m = &doc["provisioned"][&key];
        Ok(m["gentleAi"] == "5.0.0" && m["gentlePi"] == "5.0.0" && m["at"].is_string())
    };
    if ready()? {
        return Ok(());
    };
    let mut job = Job::captured(
        Spec {
            exe: runtime.join(NODE),
            args: vec![
                s(&runtime.join("gentle/node_modules/gentle-pi/bin/gentle-shell.mjs")),
                "--isolated".into(),
                "setup".into(),
            ],
            input: String::new(),
        },
        project,
        env,
    )?;
    let (ok, out) = job
        .wait(Duration::from_secs(360))
        .map_err(|e| format!("Companion setup: {e}"))?;
    if !ok || !ready()? {
        if env.contains_key("AI06_SYNTHETIC_SETUP") {
            // En este punto el proceso de setup no recibió claves de servicios ni de proveedores.
            // Emitir sólo categorías de diagnóstico, no valores de entorno, rutas ni salida cruda.
            let terms = [
                "failed",
                "error",
                "fatal",
                "warning",
                "unknown",
                "unsupported",
                "missing",
                "not installed",
                "cannot",
                "could not",
                "executable",
                "invalid",
                "permission",
                "binary",
                "package",
                "network",
                "engram",
                "project",
                "refusing",
                "invalid argument",
                "flag",
                "npm",
                "git",
                "spawn",
                "integrity",
                "EPERM",
                "ENOENT",
                "EINVAL",
                "EEXIST",
                "EACCES",
                "required",
                "exit status",
                "skip",
                "conflict",
                "outside",
                "PATH",
                "cmd.exe",
                "powershell",
                "--agent",
                "--scope",
                "preserved",
                "setup",
                "dependency",
                "timeout",
                "verify",
                "version",
                "update",
                "not recognized",
                "could not determine",
                "not set",
                "ProgramFiles",
                "SystemRoot",
            ];
            for line in out
                .lines()
                .rev()
                .take(12)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                let lower = line.to_lowercase();
                let tags = terms
                    .iter()
                    .filter(|t| lower.contains(&t.to_lowercase()))
                    .copied()
                    .collect::<Vec<_>>();
                if !tags.is_empty() {
                    eprintln!("Setup sintético: {}", tags.join(","));
                }
            }
            eprintln!("Setup sintético: exit_ok={ok}; marker={}", marker.exists());
        }
        let why = [
            "ENOENT",
            "EACCES",
            "EPERM",
            "not found",
            "preflight",
            "timed out",
            "not on PATH",
        ]
        .into_iter()
        .find(|v| out.contains(v))
        .unwrap_or("sin registro de aprovisionamiento");
        return Err(format!("Gentle Shell setup: {why}"));
    }
    Ok(())
}

pub fn run_gentle(install: std::sync::Arc<Install>, prompt: Option<String>) -> Result<()> {
    let ci = prompt.is_some();
    let runtime = install.runtime();
    let node = runtime.join(NODE);
    let bootstrap = environment(&install.temporary().join("inicio"), &node)?;
    let status = install.temporary().join("inicio/estado.txt");
    let mut splash = if !ci && !runtime.join("gentle").is_dir() {
        Some(Job::launch(Spec{exe:PathBuf::from(&bootstrap["SystemRoot"]).join("System32/WindowsPowerShell/v1.0/powershell.exe"),args:vec!["-NoProfile".into(),"-STA".into(),"-Command".into(),"Add-Type -AssemblyName System.Windows.Forms;$f=New-Object Windows.Forms.Form;$f.Text='AI Stack Desktop · Preparando';$f.Width=440;$f.Height=110;$l=New-Object Windows.Forms.Label;$l.Dock='Fill';$f.Controls.Add($l);$t=New-Object Windows.Forms.Timer;$t.Interval=750;$t.Add_Tick({try{$l.Text=[IO.File]::ReadAllText((Join-Path (Get-Location) 'estado.txt'))}catch{}});$t.Start();[void]$f.ShowDialog()".into()],input:String::new()},status.parent().unwrap(),&bootstrap)?)
    } else {
        None
    };
    let mut last = String::new();
    prepare_gentle(&install, |p| {
        let text = format!("{} · {}", p.component, p.stage);
        if last != text {
            let _ = fs::write(&status, &text);
            println!("{text}");
            last = text;
        }
    })?;
    fs::write(
        &status,
        "Preparando las cuentas locales y el companion de Gentle Shell…",
    )
    .safe()?;
    let mut sup = Supervisor::new(install.clone());
    let mut accounts = Accounts::start(&mut sup, ci).map_err(|e| format!("Cuentas: {e}"))?;
    println!("Cuentas privadas listas");
    let root = accounts.folder.join("gentle");
    install::folder(&root)?;
    let path = |name: &str| root.join(name);
    for d in [
        "agent",
        "config",
        "desktop",
        "Proyecto",
        "home",
        "memory",
        "desktop/toolchain",
        "desktop/toolchain/bin",
    ] {
        install::folder(&path(d))?;
    }
    let home = path("agent");
    let project = path("Proyecto");
    let mut env = environment(&path("home"), &node)?;
    let bin = path("desktop/toolchain/bin");
    let engram = runtime.join("engram/engram.exe");
    let go = runtime.join("go/go");
    let modules = runtime.join("gentle/node_modules");
    let mut launcher = std::env::current_exe().safe()?;
    let previous = install.root().join("bin/ai-stack-gentle.exe");
    // En un perfil 0.0.6 reutilizado, sus shims ya ofrecen el mismo contrato.
    // Conservarlos permite volver al binario original sin tocar la recuperación.
    if previous != launcher
        && install::item(&previous)?.is_some()
        && preparation::digest(&previous)?
            == "6d4c011cd36c398adad9618819dabfbb1b6a81eea11d6b493caba6d303c166ee"
    {
        launcher = previous;
    }
    for (name, source) in [
        ("node.exe", &node),
        ("go.exe", &go.join("bin/go.exe")),
        ("engram.exe", &engram),
        ("pi.exe", &launcher),
        ("setup-companion.exe", &launcher),
    ] {
        let link = bin.join(name);
        if install::item(&link)?.is_some() {
            if same_file::is_same_file(source, &link).safe()? {
                continue;
            }
            if source == &launcher && preparation::digest(&link)? == preparation::digest(source)? {
                continue;
            }
            fs::remove_file(&link).safe()?;
        }
        if fs::hard_link(source, &link).is_err() {
            // El directorio de programas elegido puede estar en otro volumen.
            if source != &launcher {
                return Err(format!("No se pudo enlazar {name}"));
            }
            atomic(&link, &fs::read(source).safe()?)?;
        }
    }
    let port = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .safe()?
        .local_addr()
        .safe()?
        .port();
    env.insert("ENGRAM_DATA_DIR".into(), s(&path("memory")));
    let mut mem = Job::launch(
        Spec {
            exe: engram.clone(),
            args: vec!["serve".into(), port.to_string()],
            input: String::new(),
        },
        &project,
        &env,
    )?;
    let mut mapi = Api::new(port);
    ready(&mut mem, &mut mapi, "/health")?;
    for (k, v) in [
        ("GENTLE_SHELL_HOME", s(&home)),
        ("PI_CODING_AGENT_DIR", s(&home)),
        ("GENTLE_SHELL_CONFIG", s(&path("launcher.json"))),
        ("AI06_RUNTIME", s(&runtime)),
        (
            "GENTLE_SHELL_GENTLE_AI_BIN",
            s(&bin.join("setup-companion.exe")),
        ),
        ("GENTLE_AI_NO_SELF_UPDATE", "1".into()),
        ("GENTLE_SHELL_ARGS", "--isolated".into()),
        ("GENTLE_SHELL_GPUI_DATA_DIR", s(&path("desktop"))),
        ("GENTLE_PI_CONFIG_HOME", s(&path("config"))),
        ("GENTLE_SHELL_NO_AUTO_SETUP", "1".into()),
        (
            "GENTLE_SHELL_BIN",
            s(&modules.join(".bin/gentle-shell.cmd")),
        ),
        ("ENGRAM_URL", mapi.origin()),
        ("ENGRAM_BIN", s(&engram)),
        ("GENTLE_AI_ENGRAM_BIN", s(&engram)),
        ("GENTLE_AI_HOME", s(&path("home/.gentle-ai"))),
        ("GENTLE_AI_PI_CONFIG_DIR", s(&path("config"))),
        ("ENGRAM_PROJECT", "gentle".into()),
        ("PI_OFFLINE", "1".into()),
        ("GOROOT", s(&go)),
        ("GOTOOLCHAIN", "local".into()),
        ("npm_config_prefix", s(&path("home/npm"))),
        ("npm_config_cache", s(&path("home/npm-cache"))),
        (
            "PATH",
            format!(
                "{};{};{};{};{}",
                s(&bin),
                s(&modules.join(".bin")),
                s(&runtime.join("git/cmd")),
                s(&runtime.join("git/usr/bin")),
                env["PATH"]
            ),
        ),
    ] {
        env.insert(k.into(), v);
    }
    if ci {
        env.insert("AI06_SYNTHETIC_SETUP".into(), "1".into());
    }
    setup(&home, &runtime, &env, &project)?;
    println!("Companion listo");
    let models = accounts.sync(&mut sup, &home)?;
    let settings = home.join("settings.json");
    let mut cfg = read_json(&settings)?;
    cfg["enabledModels"] = json!(["omniroute/*"]);
    cfg["defaultProvider"] = json!("omniroute");
    cfg["shellPath"] = json!(runtime.join("git/usr/bin/sh.exe"));
    chat_default(&mut cfg, &models);
    if ci {
        cfg["defaultThinkingLevel"] = json!("low");
    }
    write_changed(&settings, &serde_json::to_vec(&cfg).safe()?)?;
    install::folder(&home.join("extensions"))?;
    write_changed(
        &home.join("extensions/ai-stack-panel.ts"),
        include_bytes!("panel-extension.ts"),
    )?;
    let gui = path("desktop/settings.json");
    if install::item(&gui)?.is_none() {
        atomic(
            &gui,
            &serde_json::to_vec(&json!({"onboarding":{"done":true,"version":"0.1.28"}})).safe()?,
        )?;
    }
    let signal = path("sincronizar");
    accounts.controls(&bootstrap, &signal, models.is_empty() && !ci)?;
    env.insert(
        "GS_INFERENCE".into(),
        sup.services[1].api.auth[7..].to_owned(),
    );
    let mut args = vec![s(&project)];
    if let Some(prompt) = prompt {
        args.extend(["--prompt".into(), prompt]);
    }
    let mut ui = Job::launch(
        Spec {
            exe: runtime.join("gpui/gentle-shell-gpui.exe"),
            args,
            input: String::new(),
        },
        &project,
        &env,
    )?;
    let record = path("desktop/instance.json");
    let deadline = Instant::now() + Duration::from_secs(90);
    let gui_pid = loop {
        if !ui.alive() {
            return Err("GPUI terminó durante el arranque".into());
        }
        if let Ok(rec) = read_json(&record) {
            if let (Some(pid), Some(p)) = (rec["pid"].as_u64(), rec["port"].as_u64()) {
                if ui.pids()?.contains(&(pid as u32)) && ui.owns(p as u16, None)? {
                    break pid as u32;
                }
            }
        }
        if Instant::now() > deadline {
            return Err("GPUI sin listener privado".into());
        }
        sleep(Duration::from_millis(150));
    };
    if let Some(j) = splash.take() {
        j.stop()?;
    }
    println!("GPUI lista");
    let mut current_models = models;
    let mut due = Instant::now() + Duration::from_secs(12);
    while ui.alive() && ui.pids()?.contains(&gui_pid) {
        if !mem.alive()
            || sup
                .services
                .iter()
                .any(|s| s.job.pids().map_or(true, |p| p.is_empty()))
            || !accounts.panel.as_mut().unwrap().alive()
        {
            return Err("Se detuvo un componente privado".into());
        }
        if Instant::now() >= due || signal.exists() {
            let _ = fs::remove_file(&signal);
            let delay = match accounts.sync(&mut sup, &home) {
                Ok(next) => {
                    if next != current_models {
                        let mut cfg = read_json(&settings)?;
                        chat_default(&mut cfg, &next);
                        write_changed(&settings, &serde_json::to_vec(&cfg).safe()?)?;
                        current_models = next;
                    }
                    if current_models.is_empty() {
                        12
                    } else {
                        60
                    }
                }
                Err(e) if e == "HTTP 401" || e == "HTTP 403" => {
                    return Err("Acceso revocado; vuelva a abrir la aplicación".into())
                }
                Err(_) => {
                    install.log(
                        "catalogo.txt",
                        "Sincronización pendiente; se conserva el último catálogo.",
                    );
                    60
                }
            };
            due = Instant::now() + Duration::from_secs(delay);
        }
        sleep(Duration::from_secs(1));
    }
    println!("Cerrando GPUI y sus descendientes");
    ui.stop()?;
    accounts.panel.take().unwrap().stop()?;
    mem.stop()?;
    sup.stop()
}
