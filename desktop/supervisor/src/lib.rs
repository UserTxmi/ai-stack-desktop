use base64::Engine;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io,
    net::TcpListener,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};
mod http;
mod job;
use http::Api;
pub use job::worker_mode;
use job::{Job, Spec};
type Result<T> = std::result::Result<T, String>;
type Env = BTreeMap<String, String>;
trait Safe<T> {
    fn safe(self) -> Result<T>;
}
impl<T, E> Safe<T> for std::result::Result<T, E> {
    fn safe(self) -> Result<T> {
        self.map_err(|_| "Operación local fallida".to_owned())
    }
}
const TARGET: &str = "labcpa/lab-reasoner";
const PID: &str = "opencode-omniroute";
const NODE: &str = "node/node-v24.21.0-win-x64/node.exe";
const PLUGIN: &str = "peers/node_modules/@omniroute/opencode-plugin/dist/index.js";
const ROLES: [&str; 3] = ["CLIProxyAPI", "OmniRoute", "OpenCode"];
fn field(v: &Value, p: &str) -> Result<String> {
    v.pointer(p)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "Contrato JSON no satisfecho".into())
}
fn secret() -> String {
    use rand::RngCore;
    let mut bytes = [0; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn dir(p: &Path) -> Result<()> {
    fs::create_dir_all(p).safe()
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn uri(p: &Path) -> Result<String> {
    url::Url::from_file_path(p).map(|u| u.to_string()).safe()
}
fn dump(p: &Path, v: &Value) -> Result<()> {
    fs::write(p, serde_json::to_vec(v).safe()?).safe()
}
fn environment(home: &Path, node: &Path) -> Result<Env> {
    let mut e = Env::new();
    for name in [
        "SystemRoot",
        "WINDIR",
        "COMSPEC",
        "PATHEXT",
        "PROCESSOR_ARCHITECTURE",
    ] {
        if let Ok(v) = std::env::var(name) {
            e.insert(name.into(), v);
        }
    }
    let sys = PathBuf::from(e.get("SystemRoot").ok_or("SystemRoot ausente")?);
    for (k, sub) in [
        ("HOME", ""),
        ("USERPROFILE", ""),
        ("APPDATA", "roam"),
        ("LOCALAPPDATA", "local"),
        ("TEMP", "tmp"),
        ("TMP", "tmp"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_STATE_HOME", "state"),
    ] {
        let p = home.join(sub);
        dir(&p)?;
        e.insert(k.into(), s(&p));
    }
    e.insert(
        "PATH".into(),
        format!(
            "{};{};{}",
            s(node.parent().unwrap()),
            s(&sys.join("System32")),
            s(&sys)
        ),
    );
    Ok(e)
}
fn verify(root: &Path) -> Result<()> {
    if !root.join("prepared").is_file() {
        return Err("Preparar los runtimes con preparar.ps1".into());
    }
    let hashes: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../../w1.json")).unwrap();
    for (name, hash) in hashes {
        let mut h = Sha256::new();
        let mut f = File::open(root.join(&name)).map_err(|_| format!("Falta {name}"))?;
        io::copy(&mut f, &mut h).safe()?;
        if format!("{:x}", h.finalize()) != hash {
            return Err(format!("Integridad W1: {name}"));
        }
    }
    Ok(())
}
struct Service {
    job: Job,
    api: Api,
    healthy: bool,
}
#[derive(Serialize)]
pub struct Row {
    pub name: &'static str,
    pub version: &'static str,
    pub active: bool,
    pub healthy: bool,
    pub metrics: Option<(u64, f64)>,
    pub output_bytes: u64,
}
#[derive(Serialize)]
pub struct Status {
    pub preparation: String,
    pub ready: bool,
    pub services: Vec<Row>,
    pub events: Vec<String>,
    pub error: Option<String>,
}
pub struct Supervisor {
    runtime: PathBuf,
    base: PathBuf,
    profile: Option<PathBuf>,
    lock: Option<File>,
    fixture: Option<Job>,
    services: Vec<Service>,
    events: Vec<String>,
    preparation: Result<()>,
    error: Option<String>,
    last_probe: Instant,
}
impl Supervisor {
    pub fn new(runtime: PathBuf, base: PathBuf) -> Self {
        let preparation = verify(&runtime);
        Self {
            runtime,
            base,
            profile: None,
            lock: None,
            fixture: None,
            services: Vec::new(),
            events: vec![],
            preparation,
            error: None,
            last_probe: Instant::now(),
        }
    }
    fn event(&mut self, msg: String) {
        if self.events.len() == 48 {
            self.events.remove(0);
        }
        self.events.push(msg);
    }
    pub fn status(&mut self) -> Status {
        if self.last_probe.elapsed() >= Duration::from_secs(5) {
            for (i, svc) in self.services.iter_mut().enumerate() {
                svc.healthy = healthy(svc, i).is_ok();
            }
            self.last_probe = Instant::now();
        }
        let services = ROLES
            .iter()
            .enumerate()
            .map(|(i, &name)| {
                let svc = self.services.get_mut(i);
                let (active, healthy, metrics, output_bytes) = svc
                    .map(|v| {
                        (
                            v.job.alive(),
                            v.healthy,
                            v.job.metrics(),
                            v.job.bytes.load(std::sync::atomic::Ordering::Relaxed),
                        )
                    })
                    .unwrap_or_default();
                Row {
                    name,
                    version: ["8.1.7", "3.8.54", "1.19.6"][i],
                    active,
                    healthy: active && healthy,
                    metrics,
                    output_bytes,
                }
            })
            .collect();
        Status {
            preparation: self
                .preparation
                .clone()
                .err()
                .unwrap_or_else(|| "W1 verificado".into()),
            ready: self.preparation.is_ok(),
            services,
            events: self.events.clone(),
            error: self.error.clone(),
        }
    }
    pub fn start(&mut self) -> Result<()> {
        if self.lock.is_some() {
            return Err("Este supervisor ya posee el perfil".into());
        }
        self.error = None;
        let result = self.inner();
        if let Err(ref msg) = result {
            let stage = ROLES
                .get(self.services.len().saturating_sub(1))
                .unwrap_or(&"Demo");
            let text = format!("{stage}: {msg}");
            self.event(text.clone());
            self.error = Some(text);
            self.stop()?;
        }
        result
    }
    fn inner(&mut self) -> Result<()> {
        self.preparation = verify(&self.runtime);
        self.preparation.clone()?;
        dir(&self.base)?;
        self.lock = Some(
            OpenOptions::new()
                .create(true)
                .write(true)
                .share_mode(0)
                .open(self.base.join("supervisor.lock"))
                .map_err(|_| "Otra instancia posee el perfil demo o se denegó el acceso")?,
        );
        let run = self.base.join(format!("demo-{}", &secret()[..20]));
        fs::create_dir(&run).safe()?;
        self.profile = Some(run.clone());
        let project = run.join("Proyecto demo");
        dir(&project)?;
        let node = self.runtime.join(NODE);
        let plugin = self.runtime.join(PLUGIN);
        let mut reservations = Vec::new();
        for _ in 0..4 {
            reservations.push(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).safe()?)
        }
        let mut ports = Vec::new();
        for r in &reservations {
            ports.push(r.local_addr().safe()?.port())
        }
        reservations.reverse();
        let source = secret();
        let key = secret();
        let password = secret();
        let ocpass = secret();
        let demo = run.join("fixture.mjs");
        fs::write(&demo, include_str!("demo.mjs")).safe()?;
        let fix: Value =
            serde_json::from_str(include_str!("../../../experiments/phase1/fixtures.json"))
                .unwrap();
        drop(reservations.pop());
        let spec = Spec {
            exe: node.clone(),
            args: vec![s(&demo)],
            input: json!({"port":ports[0],"key":source,"fixture":fix}).to_string(),
        };
        self.fixture = Some(Job::launch(
            spec,
            &project,
            &environment(&run.join("fixture"), &node)?,
        )?);
        let mut fapi = Api::new(ports[0]);
        fapi.auth = format!("Bearer {source}");
        ready(self.fixture.as_mut().unwrap(), &mut fapi, "/v1/models")?;
        let cpa = run.join("cpa.yaml");
        dump(
            &cpa,
            &json!({"server":{"host":"127.0.0.1","port":ports[1]},"management":{"disable-control-panel":true},"access":{"api-keys":[key]},"oauth":{"auth-dir":run.join("cpa-auth")},"api-keys":{"openai-compatibility":[{"name":"fixture","base-url":format!("{}/v1",fapi.origin()),"keys":[{"api-key":source}],"models":[{"name":"lab-reasoner","alias":"lab-reasoner","thinking":{"levels":fix["efforts"]}}]}]}}),
        )?;
        let mut api = Api::new(ports[1]);
        api.auth = format!("Bearer {key}");
        drop(reservations.pop());
        self.launch(
            self.runtime.join("cpa/cli-proxy-api.exe"),
            vec!["-config".into(), s(&cpa)],
            &project,
            environment(&run.join("cpa"), &node)?,
            api,
            "/v1/models",
        )?;
        healthy(&mut self.services[0], 0)?;
        let gateway = Api::new(ports[2]);
        let origin = gateway.origin();
        let omni = self.runtime.join("package/dist");
        let mut env = environment(&run.join("omni"), &node)?;
        for (k, v) in [
            ("PORT", ports[2].to_string()),
            ("HOSTNAME", "127.0.0.1".into()),
            ("OMNIROUTE_SERVER_HOST", "127.0.0.1".into()),
            ("DATA_DIR", s(&run.join("omni-data"))),
            ("NODE_ENV", "production".into()),
            ("INITIAL_PASSWORD", password.clone()),
            ("REQUIRE_API_KEY", "true".into()),
            ("JWT_SECRET", secret()),
            ("STORAGE_ENCRYPTION_KEY", secret()),
            ("NEXT_PUBLIC_BASE_URL", origin.clone()),
            ("NEXT_TELEMETRY_DISABLED", "1".into()),
            ("APP_LOG_TO_FILE", "false".into()),
            ("ENABLE_REQUEST_LOGS", "false".into()),
            ("CALL_LOG_PIPELINE_CAPTURE_STREAM_CHUNKS", "false".into()),
        ] {
            env.insert(k.into(), v);
        }
        drop(reservations.pop());
        self.launch(
            node.clone(),
            vec![s(&omni.join("server-ws.mjs"))],
            &omni,
            env,
            gateway,
            "/healthz",
        )?;
        let (c, g) = self.services.split_at_mut(1);
        let gw = &mut g[0];
        let token = gw.api.provision(&gw.job, &c[0].api, &password)?;
        gw.api = Api::new(ports[2]);
        gw.api.auth = format!("Bearer {token}");
        let row = healthy(gw, 1)?;
        let project_env = environment(&run.join("oc"), &node)?;
        let data = PathBuf::from(&project_env["XDG_DATA_HOME"]).join("opencode");
        dir(&data)?;
        dump(
            &data.join("auth.json"),
            &json!({(PID):{"type":"api","key":token}}),
        )?;
        let conf = run.join("opencode.json");
        let helper = run.join("catalog.mjs");
        fs::write(&helper, include_str!("catalog.mjs")).safe()?;
        let mut catalog=Job::launch(Spec{exe:node.clone(),args:vec![s(&helper)],input:json!({"plugin":plugin,"baseURL":origin,"apiKey":token,"target":TARGET,"row":row,"file":conf}).to_string()},&project,&project_env)?;
        for _ in 0..200 {
            if !catalog.alive() {
                break;
            }
            sleep(Duration::from_millis(50))
        }
        if catalog.alive() {
            return Err("Plazo de proyección del catálogo".into());
        }
        let overlay: Value = serde_json::from_slice(
            &fs::read(&conf).map_err(|_| "Proyección del catálogo W1 fallida")?,
        )
        .safe()?;
        dump(
            &conf,
            &json!({"plugin":[[uri(&plugin)?,{"providerId":"omniroute","baseURL":origin,"autoSyncIntervalMs":0,"features":{"combos":false,"autoCombos":false,"enrichment":false,"diskCache":false,"mcpAutoEmit":false}}],[uri(&self.runtime.join("peers/selection.mjs"))?,{"providerID":PID,"modelID":TARGET,"efforts":overlay["tiers"],"rejected":run.join("rejected.flag")}]],"provider":overlay["provider"],"permission":"ask","enabled_providers":[PID]}),
        )?;
        let mut env = project_env;
        for (k, v) in [
            ("OPENCODE_CONFIG", s(&conf)),
            ("OPENCODE_SERVER_PASSWORD", ocpass.clone()),
            ("OPENCODE_DATA_DIR", s(&data)),
            ("OPENCODE_DISABLE_AUTOUPDATE", "1".into()),
            ("OPENCODE_DISABLE_MODELS_FETCH", "1".into()),
        ] {
            env.insert(k.into(), v);
        }
        let mut agent = Api::new(ports[3]);
        agent.auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("opencode:{ocpass}"))
        );
        drop(reservations.pop());
        self.launch(
            self.runtime.join("oc/opencode.exe"),
            vec![
                "serve".into(),
                "--hostname".into(),
                "127.0.0.1".into(),
                "--port".into(),
                ports[3].to_string(),
            ],
            &project,
            env,
            agent,
            "/global/health",
        )?;
        let a = &mut self.services[2];
        healthy(a, 2)?;
        self.event("OpenCode: salud autenticada; cargando catálogo del proyecto".into());
        let a = &mut self.services[2];
        let providers = a.api.request(&a.job, "GET", "/provider", Value::Null)?;
        let variants = providers["all"]
            .as_array()
            .and_then(|ps| ps.iter().find(|p| p["id"] == PID))
            .and_then(|p| p["models"][TARGET]["variants"].as_object())
            .ok_or("Modelo demo ausente")?;
        if variants.len() != 2 || !variants.contains_key("low") || !variants.contains_key("high") {
            return Err("Variantes W1 incorrectas".into());
        }
        self.event("Catálogo OpenCode confirmado: low / high".into());
        Ok(())
    }
    fn launch(
        &mut self,
        exe: PathBuf,
        args: Vec<String>,
        cwd: &Path,
        env: Env,
        api: Api,
        path: &str,
    ) -> Result<()> {
        let role = ROLES[self.services.len()];
        let job = Job::launch(
            Spec {
                exe,
                args,
                input: String::new(),
            },
            cwd,
            &env,
        )?;
        self.services.push(Service {
            job,
            api,
            healthy: false,
        });
        let svc = self.services.last_mut().unwrap();
        ready(&mut svc.job, &mut svc.api, path)?;
        svc.healthy = true;
        self.event(format!("{role}: listener propio y respuesta comprobados"));
        Ok(())
    }
    /// La prueba de CI comprueba inferencia desde OpenCode, no sólo procesos escuchando.
    pub fn demo_turn(&mut self) -> Result<()> {
        let a = self.services.get_mut(2).ok_or("OpenCode detenido")?;
        let id = field(
            &a.api.request(&a.job, "POST", "/session", json!({}))?,
            "/id",
        )?;
        if !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return Err("ID de sesión inválido".into());
        }
        let out=a.api.request(&a.job,"POST",&format!("/session/{id}/message"),json!({"model":{"providerID":PID,"modelID":TARGET},"variant":"high","parts":[{"type":"text","text":"W1-TEST:high:END"}]}))?;
        if !out["info"]["error"].is_null()
            || !out["parts"].as_array().is_some_and(|ps| {
                ps.iter().any(|p| {
                    p["type"] == "text" && p["text"].as_str().is_some_and(|t| t.contains("LAB"))
                })
            })
        {
            return Err("Turno sin respuesta sintética".into());
        }
        self.event("Turno sintético completado".into());
        Ok(())
    }
    pub fn stop(&mut self) -> Result<()> {
        let mut err = None;
        while let Some(mut svc) = self.services.pop() {
            let name = ROLES[self.services.len()];
            if let Err(e) = svc.job.stop() {
                err = Some(e)
            } else {
                self.event(format!("{name}: Job vacío"));
            }
        }
        if let Some(mut j) = self.fixture.take() {
            if let Err(e) = j.stop() {
                err = Some(e)
            }
        }
        if let Some(p) = self.profile.take() {
            for _ in 0..20 {
                if fs::remove_dir_all(&p).is_ok() {
                    break;
                }
                sleep(Duration::from_millis(100))
            }
            if p.exists() {
                err = Some("No se pudo limpiar el perfil demo".into())
            }
        }
        self.lock.take();
        if let Some(e) = err {
            self.error = Some(e.clone());
            Err(e)
        } else {
            Ok(())
        }
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
fn ready(job: &mut Job, api: &mut Api, path: &str) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(90);
    let mut last = "Sin listener".to_string();
    while Instant::now() < until {
        if !job.alive() {
            return Err("Proceso terminó durante el arranque".into());
        }
        if job.owns(api.port, None)? {
            match api.request(job, "GET", path, Value::Null) {
                Ok(_) => return Ok(()),
                Err(e) => last = e,
            }
        }
        sleep(Duration::from_millis(250))
    }
    Err(format!("Sonda agotada: {last}"))
}
fn healthy(svc: &mut Service, i: usize) -> Result<Value> {
    let path = if i == 2 {
        "/global/health"
    } else {
        "/v1/models"
    };
    let v = svc.api.request(&svc.job, "GET", path, Value::Null)?;
    if i == 2 {
        return if v["healthy"] == true {
            Ok(v)
        } else {
            Err("OpenCode no sano".into())
        };
    }
    let target = if i == 0 { "lab-reasoner" } else { TARGET };
    v["data"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["id"] == target))
        .cloned()
        .ok_or_else(|| "Catálogo sin modelo demo".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_adopta_listener_ajeno() {
        let foreign = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = foreign.local_addr().unwrap().port();
        let raw = unsafe {
            windows_sys::Win32::System::JobObjects::CreateJobObjectW(
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert!(!raw.is_null());
        job::test_foreign(raw, port);
        assert!(foreign.local_addr().is_ok());
    }
}
