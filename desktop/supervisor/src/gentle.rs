use super::*;

/// Entrada dedicada: preparación y servicios comunes; nunca entra en la rama OpenCode.
pub fn run_gentle(install: std::sync::Arc<Install>, prompt: Option<String>) -> Result<()> {
    let probe = prompt.as_deref() == Some("LAB-DELEGAR");
    prepare_gentle(&install, |p| println!("{} · {}", p.component, p.stage))?;
    let mut sup = Supervisor::new(install.clone());
    sup.inner(true)?;
    let runtime = install.runtime();
    let node = runtime.join(NODE);
    let root = install.root().join("datos/gentle");
    install::folder(&root)?;
    let path = |name: &str| root.join(name);
    for d in [
        "agent",
        "config",
        "desktop",
        "Proyecto demo",
        "home",
        "memory",
    ] {
        install::folder(&path(d))?;
    }
    let home = path("agent");
    let config = path("config");
    let project = path("Proyecto demo");
    let gui_settings = path("desktop/settings.json");
    if install::item(&gui_settings)?.is_none() {
        dump(
            &gui_settings,
            &json!({"onboarding":{"done":true,"version":"0.1.28"}}),
        )?;
    }
    let note = path("nota-demo.txt");
    if install::item(&note)?.is_none() {
        fs::write(note, "G2-CONTENIDO").safe()?;
    }
    let mut env = environment(&path("home"), &node)?;
    let engram = runtime.join("engram/engram.exe");
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
    let gw = &mut sup.services[1];
    let catalog = gw.api.request(&gw.job, "GET", "/v1/models", Value::Null)?;
    let mut models = Vec::new();
    for row in catalog["data"].as_array().ok_or("Catálogo ausente")? {
        // La única conexión autorizada de este portable es el fixture CPA.
        if row["id"] != TARGET && row["id"] != "labcpa/lab-helper" {
            continue;
        }
        let tiers = row["capabilities"]["effort_tiers"]
            .as_array()
            .ok_or("Faltan esfuerzos")?;
        let mut map = serde_json::Map::new();
        for t in ["off", "minimal", "low", "medium", "high", "xhigh", "max"] {
            map.insert(
                t.into(),
                if tiers.iter().any(|v| v == t) {
                    json!(t)
                } else {
                    Value::Null
                },
            );
        }
        models.push(json!({"id":row["id"],"reasoning":true,"thinkingLevelMap":map,"contextWindow":row["context_length"],"maxTokens":row["max_output_tokens"]}));
    }
    if models.len() != 2 {
        return Err("Modelo del proveedor sintético ausente".into());
    }
    let modules = runtime.join("gentle/node_modules");
    dump(
        &home.join("models.json"),
        &json!({"providers":{"omniroute":{"api":"openai-completions","baseUrl":format!("{}/v1",gw.api.origin()),"apiKey":"$GS_INFERENCE","compat":{"supportsReasoningEffort":true,"thinkingFormat":"openai"},"models":models}}}),
    )?;
    let settings = home.join("settings.json");
    if install::item(&settings)?.is_none() {
        dump(
            &settings,
            &json!({"defaultProvider":"omniroute","defaultModel":TARGET,"defaultThinkingLevel":"low","enabledModels":["omniroute/*"],"shellPath":runtime.join("git/usr/bin/sh.exe"),"packages":[modules.join("gentle-engram")]}),
        )?;
    }
    let routing = config.join("models.json");
    if install::item(&routing)?.is_none() {
        dump(
            &routing,
            &json!({"gentle-ai-explore":{"model":"omniroute/labcpa/lab-helper","thinking":"high"}}),
        )?;
    }
    let bin = path("desktop/toolchain/bin");
    fs::create_dir_all(&bin).safe()?;
    // GPUI antepone toolchain/bin incluso al PATH de login leído del registro.
    // Enlace duro NTFS creado por el propietario; no resolver `node` en PATH del usuario.
    let private_node = bin.join("node.exe");
    if install::item(&private_node)?.is_none() {
        fs::hard_link(&node, &private_node).safe()?;
    }
    if preparation::digest(&private_node)? != preparation::digest(&node)? {
        return Err("Node privado de GPUI no coincide".into());
    }
    for (key, value) in [
        ("GENTLE_SHELL_HOME", s(&home)),
        ("PI_CODING_AGENT_DIR", s(&home)),
        ("GENTLE_SHELL_ARGS", "--isolated".into()),
        ("GENTLE_SHELL_GPUI_DATA_DIR", s(&path("desktop"))),
        ("GENTLE_PI_CONFIG_HOME", s(&config)),
        ("GENTLE_SHELL_NO_AUTO_SETUP", "1".into()),
        (
            "GENTLE_SHELL_BIN",
            s(&modules.join(".bin/gentle-shell.cmd")),
        ),
        ("ENGRAM_URL", mapi.origin()),
        ("ENGRAM_BIN", s(&engram)),
        ("PI_OFFLINE", "1".into()),
        ("ENGRAM_PROJECT", "gentle-demo".into()),
        ("GS_INFERENCE", gw.api.auth[7..].to_string()),
        (
            "PATH",
            format!(
                "{};{};{}",
                s(&runtime.join("git/cmd")),
                s(&runtime.join("git/usr/bin")),
                env["PATH"]
            ),
        ),
    ] {
        env.insert(key.into(), value);
    }
    let mut args = vec![s(&project)];
    if let Some(text) = prompt {
        args.extend(["--prompt".into(), text]);
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
    loop {
        if !ui.alive() {
            return Err("GPUI terminó antes de adquirir su instancia privada".into());
        }
        if let Ok(rec) = fs::read(&record)
            .and_then(|b| serde_json::from_slice::<Value>(&b).map_err(io::Error::other))
        {
            if let (Some(pid), Some(p)) = (rec["pid"].as_u64(), rec["port"].as_u64()) {
                if ui.pids()?.contains(&(pid as u32)) && ui.owns(p as u16, None)? {
                    break;
                }
            }
        }
        if Instant::now() > deadline {
            return Err("GPUI sin listener privado".into());
        }
        sleep(Duration::from_millis(150));
    }
    println!("GPUI lista. Cierre su ventana para detener el stack.");
    let receipt = install
        .temporary()
        .join(sup.profile.as_ref().unwrap())
        .join("hijo-activo.txt");
    let mut probed = false;
    while ui.alive() {
        if !mem.alive()
            || sup
                .services
                .iter()
                .any(|v| v.job.pids().map_or(true, |p| p.is_empty()))
        {
            return Err("Se detuvo un servicio del stack".into());
        }
        if probe && !probed && fs::read_to_string(&receipt).is_ok_and(|v| v == "high") {
            // La prueba necesita la identidad de los Jobs antes de matar a su propietario,
            // no una reconstrucción a partir de PIDs padre que Windows puede reutilizar.
            dump(
                &receipt.with_file_name("jobs-ci.json"),
                &json!({
                    "servicios":sup.owned_pids()?, "engram":mem.pids()?, "gpui":ui.pids()?
                }),
            )?;
            probed = true;
        }
        sleep(Duration::from_millis(300));
    }
    ui.stop()?;
    mem.stop()?;
    sup.stop()
}
