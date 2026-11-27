//! Perfil de cuentas 0.0.6. Ninguna ruta persistente entra en la limpieza de la demo.
use super::*;
use serde::Deserialize;
use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};

fn crypt(input: &[u8], seal: bool) -> Result<Vec<u8>> {
    let src = CRYPT_INTEGER_BLOB {
        cbData: input.len().try_into().safe()?,
        pbData: input.as_ptr() as _,
    };
    unsafe {
        let mut dst = std::mem::zeroed::<CRYPT_INTEGER_BLOB>();
        let ok = if seal {
            CryptProtectData(
                &src,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut dst,
            )
        } else {
            CryptUnprotectData(
                &src,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut dst,
            )
        };
        if ok == 0 {
            return Err("Windows no pudo abrir el almacén DPAPI de este usuario".into());
        }
        let data = std::slice::from_raw_parts(dst.pbData, dst.cbData as usize).to_vec();
        LocalFree(dst.pbData.cast());
        Ok(data)
    }
}

pub(super) fn atomic(path: &Path, data: &[u8]) -> Result<()> {
    use std::io::Write;
    install::item(path)?;
    let tmp = path.with_extension(format!("tmp-{}", &secret()[..12]));
    let mut f = File::create_new(&tmp).safe()?;
    f.write_all(data).and_then(|_| f.sync_all()).safe()?;
    drop(f);
    fs::rename(&tmp, path).safe()
}
fn powershell(env: &Env) -> PathBuf {
    PathBuf::from(&env["SystemRoot"]).join("System32/WindowsPowerShell/v1.0/powershell.exe")
}

/// ACL heredable aplicada antes de crear archivos OAuth, SQLite o configuración.
fn private_dir(path: &Path) -> Result<()> {
    use std::os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    };
    use windows_sys::Win32::{
        Security::{Authorization::*, *},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    install::folder(path)?;
    unsafe {
        let mut handle = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) == 0 {
            return Err("No se pudo identificar al usuario de Windows".into());
        }
        let token = OwnedHandle::from_raw_handle(handle);
        let mut size = 0;
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            std::ptr::null_mut(),
            0,
            &mut size,
        );
        let mut info = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
        if GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            info.as_mut_ptr().cast(),
            size,
            &mut size,
        ) == 0
        {
            return Err("No se pudo leer el SID del usuario".into());
        }
        let sid = (*(info.as_ptr().cast::<TOKEN_USER>())).User.Sid;
        let mut ace = std::mem::zeroed::<EXPLICIT_ACCESS_W>();
        ace.grfAccessPermissions = 0x1f01ff;
        ace.grfAccessMode = SET_ACCESS;
        ace.grfInheritance = OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE;
        BuildTrusteeWithSidW(&mut ace.Trustee, sid);
        let mut acl = std::ptr::null_mut();
        if SetEntriesInAclW(1, &ace, std::ptr::null(), &mut acl) != 0 {
            return Err("No se pudo construir la ACL privada".into());
        }
        let name: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let code = SetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            acl,
            std::ptr::null_mut(),
        );
        LocalFree(acl.cast());
        if code != 0 {
            return Err(format!("No se pudo aplicar la ACL privada ({code})"));
        }
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct Vault {
    ports: [u16; 2],
    cpa: String,
    admin: String,
    password: String,
    storage: String,
    jwt: String,
    token: String,
    connection: String,
    owner: String,
}
impl Vault {
    fn save(&self, path: &Path) -> Result<()> {
        atomic(path, &crypt(&serde_json::to_vec(self).safe()?, true)?)
    }
}
pub(super) struct Accounts {
    v: Vault,
    admin: Api,
    last: Value,
    pub panel: Option<Job>,
    pub folder: PathBuf,
}
fn rows<'a>(value: &'a Value, k: &str) -> Result<&'a Vec<Value>> {
    value[k]
        .as_array()
        .ok_or_else(|| "Lista de integración no válida".into())
}
impl Accounts {
    pub fn start(sup: &mut Supervisor, ci: bool) -> Result<Self> {
        let install = sup.install.clone();
        let node = install.runtime().join(NODE);
        let name = if ci { "cuentas-ci" } else { "cuentas" };
        let folder = install.root().join("datos").join(name);
        let env = environment(&install.temporary().join("acl"), &node)?;
        private_dir(&folder)?;
        for dir in ["auth", "omni", "home-cpa", "home-omni"] {
            install::folder(&folder.join(dir))?;
        }
        let vault = folder.join("claves.dpapi");
        let bind = |p| {
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, p)).map_err(|_| {
                "Puerto del perfil ocupado. Cierre la aplicación que lo usa y vuelva a abrir."
            })
        };
        let mut v: Vault = if install::item(&vault)?.is_some() {
            serde_json::from_slice(&crypt(&fs::read(&vault).safe()?, false)?).safe()?
        } else {
            let a = bind(0)?;
            let b = bind(0)?;
            let v = Vault {
                ports: [a.local_addr().safe()?.port(), b.local_addr().safe()?.port()],
                cpa: secret(),
                admin: secret(),
                password: secret(),
                storage: secret(),
                jwt: secret(),
                token: String::new(),
                connection: String::new(),
                owner: format!("AI06-{}", &secret()[..16]),
            };
            v.save(&vault)?;
            v
        };
        let cport = bind(v.ports[0])?;
        let gport = bind(v.ports[1])?;
        let mut c = Api::new(v.ports[0]);
        c.auth = format!("Bearer {}", v.cpa);
        let config = folder.join("cpa.yaml");
        let mut initial = json!({"server":{"host":"127.0.0.1","port":c.port},"management":{"secret-key":v.admin,"allow-remote":false,"disable-auto-update-panel":true},"access":{"api-keys":[v.cpa]},"oauth":{"auth-dir":folder.join("auth")},"routing":{"strategy":"fill-first","session-affinity":true,"session-affinity-rebind-on":"never","retry":{"request-retry":0,"max-retry-credentials":1},"cooldown":{"disable-cooling":false,"save-cooldown-status":true}}});
        if ci {
            // Se añade únicamente en el perfil desechable de Actions; nunca se abre el perfil real.
            let f = bind(0)?;
            let port = f.local_addr().safe()?.port();
            drop(f);
            let key = secret();
            let fixture = folder.join("fixture.mjs");
            fs::write(&fixture, include_str!("demo.mjs")).safe()?;
            let spec=Spec{exe:node.clone(),args:vec![s(&fixture)],input:json!({"port":port,"key":key,"gentle":true,"fixture":serde_json::from_str::<Value>(include_str!("../../../experiments/phase1/fixtures.json")).unwrap()}).to_string()};
            let mut job = Job::launch(spec, &folder, &env)?;
            let mut api = Api::new(port);
            api.auth = format!("Bearer {key}");
            ready(&mut job, &mut api, "/v1/models")?;
            sup.fixture = Some(job);
            initial["api-keys"] = json!({"openai-compatibility":[{"name":"fixture","base-url":format!("{}/v1",api.origin()),"keys":[{"api-key":key}],"models":[{"name":"lab-reasoner","alias":"lab-reasoner","thinking":{"levels":["low","high"]}}]}]});
        }
        if ci || install::item(&config)?.is_none() {
            atomic(&config, &serde_json::to_vec(&initial).safe()?)?;
        }
        let mut env = environment(&folder.join("home-cpa"), &node)?;
        env.insert(
            "MANAGEMENT_STATIC_PATH".into(),
            s(&install.runtime().join("panel")),
        );
        drop(cport);
        sup.launch(
            install.runtime().join("cpa/cli-proxy-api.exe"),
            vec!["-config".into(), s(&config)],
            &folder,
            env,
            c,
            "/v1/models",
        )?;
        let g = Api::new(v.ports[1]);
        let origin = g.origin();
        let mut env = environment(&folder.join("home-omni"), &node)?;
        for (k, val) in [
            ("PORT", g.port.to_string()),
            ("HOSTNAME", "127.0.0.1".into()),
            ("OMNIROUTE_SERVER_HOST", "127.0.0.1".into()),
            ("DATA_DIR", s(&folder.join("omni"))),
            ("NODE_ENV", "production".into()),
            ("INITIAL_PASSWORD", v.password.clone()),
            ("REQUIRE_API_KEY", "true".into()),
            ("STORAGE_ENCRYPTION_KEY", v.storage.clone()),
            ("JWT_SECRET", v.jwt.clone()),
            ("NEXT_PUBLIC_BASE_URL", origin),
            ("NEXT_TELEMETRY_DISABLED", "1".into()),
            ("APP_LOG_TO_FILE", "false".into()),
            ("ENABLE_REQUEST_LOGS", "false".into()),
            ("CALL_LOG_PIPELINE_CAPTURE_STREAM_CHUNKS", "false".into()),
        ] {
            env.insert(k.into(), val);
        }
        let dist = install.runtime().join("package/dist");
        drop(gport);
        sup.launch(
            node,
            vec![s(&dist.join("server-ws.mjs"))],
            &dist,
            env,
            g,
            "/healthz",
        )?;
        let job = &sup.services[1].job;
        let mut admin = Api::new(v.ports[1]);
        admin.login(job, &v.password)?;
        let origin = format!("{}/v1", sup.services[0].api.origin());
        let nodes = admin.request(job, "GET", "/api/provider-nodes", Value::Null)?;
        let matches: Vec<_> = rows(&nodes, "nodes")?
            .iter()
            .filter(|n| n["prefix"] == "cpa")
            .collect();
        let nid=match matches.as_slice(){[]=>field(&admin.request(job,"POST","/api/provider-nodes",json!({"type":"openai-compatible","apiType":"chat","name":v.owner,"prefix":"cpa","baseUrl":origin}))?,"/node/id")?,[n] if n["name"]==v.owner && n["baseUrl"]==origin =>field(n,"/id")?,_=>return Err("Conflicto con el nodo CPA gestionado".into())};
        let connections = admin.request(job, "GET", "/api/providers", Value::Null)?;
        let matches: Vec<_> = rows(&connections, "connections")?
            .iter()
            .filter(|n| n["provider"] == nid)
            .collect();
        let id = match matches.as_slice() {
            [] if v.connection.is_empty() => field(
                &admin.request(
                    job,
                    "POST",
                    "/api/providers",
                    json!({"provider":nid,"name":v.owner,"apiKey":v.cpa}),
                )?,
                "/connection/id",
            )?,
            [n] if n["name"] == v.owner => field(n, "/id")?,
            _ => return Err("Conexión CPA ausente o ambigua".into()),
        };
        if !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
            || (!v.connection.is_empty() && v.connection != id)
        {
            return Err("Cambió la identidad de la conexión CPA".into());
        }
        if v.token.is_empty() {
            admin.request(job,"PATCH","/api/resilience",json!({"requestQueue":{"maxWaitMs":15000,"executionMaxWaitMs":90000,"concurrentRequests":1,"autoEnableApiKeyProviders":true}}))?;
            admin.request(
                job,
                "POST",
                "/api/rate-limits",
                json!({"connectionId":id,"enabled":true}),
            )?;
            let keys = admin.request(job, "GET", "/api/keys", Value::Null)?;
            for k in rows(&keys, "keys")?.iter().filter(|k| k["name"] == v.owner) {
                let kid = field(k, "/id")?;
                if !kid
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
                {
                    return Err("ID de clave inválido".into());
                }
                admin.request(job, "DELETE", &format!("/api/keys/{kid}"), Value::Null)?;
            }
            v.token = field(
                &admin.request(
                    job,
                    "POST",
                    "/api/keys",
                    json!({"name":v.owner,"allowedConnections":[id],"noLog":true}),
                )?,
                "/key",
            )?;
            v.connection = id;
            v.save(&vault)?;
        }
        sup.services[1].api.auth = format!("Bearer {}", v.token);
        let mut management = Api::new(v.ports[0]);
        management.auth = format!("Bearer {}", v.admin);
        if ci {
            let job = &sup.services[0].job;
            management.request(job, "GET", "/v8/management/credentials", Value::Null)?;
            let oauth = management.request(
                job,
                "POST",
                "/v8/management/oauth/auth-url",
                json!({"provider":"codex"}),
            )?;
            let url = url::Url::parse(&field(&oauth, "/url")?).safe()?;
            let state = field(&oauth, "/state")?;
            if url.scheme() != "https"
                || url.host_str() != Some("auth.openai.com")
                || !state
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
            {
                return Err("Inicio OAuth sintético inesperado".into());
            }
            let cancelled = management.request(
                job,
                "DELETE",
                &format!("/v8/management/oauth/session?state={state}"),
                Value::Null,
            )?;
            if cancelled["cancelled"] != true {
                return Err("La sesión OAuth de ensayo no se canceló".into());
            }
        }
        Ok(Self {
            v,
            admin,
            last: Value::Null,
            panel: None,
            folder,
        })
    }
    pub fn sync(&mut self, sup: &mut Supervisor, home: &Path) -> Result<Vec<Value>> {
        let (c, g) = sup.services.split_at_mut(1);
        let c = &mut c[0];
        let g = &mut g[0];
        let live = c
            .api
            .request(&c.job, "GET", "/v1/models?format=grok-shell", Value::Null)?;
        let available = rows(&live, "data")?;
        if live != self.last && !available.is_empty() {
            self.admin.request(
                &g.job,
                "POST",
                &format!(
                    "/api/providers/{}/sync-models?mode=import&quiet=1",
                    self.v.connection
                ),
                json!({}),
            )?;
        }
        let gateway = g.api.request(&g.job, "GET", "/v1/models", Value::Null)?;
        let mut models = Vec::new();
        for r in available {
            let raw = field(r, "/id")?;
            let id = format!("cpa/{raw}");
            let Some(grow) = rows(&gateway, "data")?.iter().find(|m| m["id"] == id) else {
                continue;
            };
            let tiers = r["reasoning_efforts"].as_array();
            let mut map = serde_json::Map::new();
            for t in ["off", "minimal", "low", "medium", "high", "xhigh", "max"] {
                map.insert(
                    t.into(),
                    if tiers.is_some_and(|a| a.iter().any(|v| v["value"] == t)) {
                        json!(t)
                    } else {
                        Value::Null
                    },
                );
            }
            if live != self.last && tiers.is_some() {
                let levels = map
                    .values()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",");
                if !levels.is_empty() {
                    self.admin.request(
                        &g.job,
                        "PATCH",
                        "/api/model-capability-overrides",
                        json!({"target":id,"key":"reasoning_efforts","value":levels}),
                    )?;
                }
            }
            let mut model = json!({"id":id,"name":id,"reasoning":map.values().any(|v|!v.is_null()),"thinkingLevelMap":map});
            for (k, a, b) in [
                ("contextWindow", "context_window", "context_length"),
                ("maxTokens", "max_completion_tokens", "max_output_tokens"),
            ] {
                if let Some(n) = r[a].as_u64().or(grow[b].as_u64()).filter(|n| *n > 0) {
                    model[k] = json!(n);
                }
            }
            models.push(model);
        }
        models.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        let doc = json!({"providers":{"omniroute":{"api":"openai-completions","baseUrl":format!("{}/v1",g.api.origin()),"apiKey":"$GS_INFERENCE","compat":{"supportsReasoningEffort":true,"thinkingFormat":"openai"},"models":models}}});
        let bytes = serde_json::to_vec(&doc).safe()?;
        let dest = home.join("models.json");
        if fs::read(&dest).ok().as_ref() != Some(&bytes) {
            atomic(&dest, &bytes)?;
        }
        self.last = live;
        Ok(models)
    }
    pub fn controls(&mut self, env: &Env, signal: &Path, open: bool) -> Result<()> {
        let script = include_str!("panel.ps1");
        self.panel=Some(Job::launch(Spec{exe:powershell(env),args:vec!["-NoProfile".into(),"-NonInteractive".into(),"-STA".into(),"-Command".into(),script.into()],input:json!({"cpa":self.v.ports[0],"omni":self.v.ports[1],"admin":self.v.admin,"password":self.v.password,"signal":signal,"open":open}).to_string()},&self.folder,env)?);
        Ok(())
    }
}
