//! HTTP/1 sin proxy ni redirecciones. Validar el socket conectado antes de escribir.
use super::*;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{body::Bytes, client::conn::http1, Request};
use hyper_util::rt::TokioIo;
pub(crate) struct Api {
    pub port: u16,
    pub auth: String,
    cookies: BTreeMap<String, String>,
    csrf: String,
}
impl Api {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            auth: String::new(),
            cookies: BTreeMap::new(),
            csrf: String::new(),
        }
    }
    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
    pub fn request(&mut self, job: &Job, method: &str, path: &str, body: Value) -> Result<Value> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .safe()?;
        rt.block_on(async {
            // /provider inicializa el proyecto y carga plugins en frío. run.py usa
            // el plazo de operación de 90 s, distinto de una sonda de salud.
            let timeout = if path == "/provider" || path.ends_with("/message") {
                90
            } else {
                12
            };
            tokio::time::timeout(Duration::from_secs(timeout), async {
                let socket =
                    tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, self.port))
                        .await
                        .safe()?;
                let peer = Some(socket.local_addr().safe()?.port());
                // La tabla puede crecer entre las dos lecturas. Sólo reintentar la
                // observación del mismo socket, nunca reproducir un POST.
                for attempt in 0..5 {
                    if job.owns(self.port, peer)? {
                        break;
                    }
                    if attempt == 4 {
                        return Err("Propiedad TCP no confirmada".into());
                    }
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
                let (mut tx, conn) = http1::handshake(TokioIo::new(socket)).await.safe()?;
                tokio::spawn(conn);
                let origin = self.origin();
                let mut req = Request::builder()
                    .method(method)
                    .uri(path)
                    .header("host", format!("127.0.0.1:{}", self.port))
                    .header("origin", origin)
                    .header("content-type", "application/json");
                // La proyección local de CPA 8.1.7 para este formato devuelve los niveles
                // del registro activo, incluidos alias; no se envía a proveedores externos.
                if path == "/v1/models?format=grok-shell" {
                    req = req.header(
                        "user-agent",
                        "AI-Stack-Desktop/0.0.6 (grok-shell catalog format)",
                    );
                }
                if !self.auth.is_empty() {
                    req = req.header("authorization", &self.auth)
                }
                if !self.csrf.is_empty() {
                    req = req.header("x-omniroute-csrf", &self.csrf)
                }
                if !self.cookies.is_empty() {
                    req = req.header(
                        "cookie",
                        self.cookies
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join("; "),
                    )
                }
                let bytes = if body.is_null() {
                    Vec::new()
                } else {
                    serde_json::to_vec(&body).safe()?
                };
                let resp = tx
                    .send_request(req.body(Full::new(Bytes::from(bytes))).safe()?)
                    .await
                    .safe()?;
                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    // Sólo rutas estáticas en el diagnóstico; nunca query/estado OAuth ni IDs privados.
                    if [
                        "/api/providers",
                        "/api/provider-nodes",
                        "/api/keys",
                        "/api/resilience",
                        "/api/rate-limits",
                        "/api/auth/login",
                        "/api/model-capability-overrides",
                        "/v8/management/oauth/auth-url",
                    ]
                    .contains(&path)
                    {
                        return Err(format!("{method} {path}: HTTP {status}"));
                    }
                    return Err(format!("HTTP {status}"));
                }
                for h in resp.headers().get_all("set-cookie") {
                    if let Some((k, v)) = h
                        .to_str()
                        .ok()
                        .and_then(|s| s.split(';').next())
                        .and_then(|v| v.split_once('='))
                    {
                        self.cookies.insert(k.into(), v.into());
                    }
                }
                let data = Limited::new(resp.into_body(), 8 << 20)
                    .collect()
                    .await
                    .safe()?
                    .to_bytes();
                if path == "/healthz" {
                    return if data.trim_ascii() == b"ok" {
                        Ok(json!(true))
                    } else {
                        Err("Gateway no listo".into())
                    };
                }
                serde_json::from_slice(&data).safe()
            })
            .await
            .map_err(|_| {
                format!(
                    "Plazo HTTP de {timeout} s agotado ({})",
                    if path == "/provider" {
                        "catálogo OpenCode"
                    } else if path.ends_with("/message") {
                        "turno"
                    } else {
                        "sonda o gestión"
                    }
                )
            })?
        })
    }
    pub(super) fn login(&mut self, job: &Job, password: &str) -> Result<()> {
        self.request(job, "POST", "/api/auth/login", json!({"password":password}))?;
        self.csrf = field(
            &self.request(job, "GET", "/api/auth/csrf", Value::Null)?,
            "/token",
        )?;
        Ok(())
    }
    pub fn provision(
        &mut self,
        job: &Job,
        cpa: &Api,
        password: &str,
        gentle: bool,
    ) -> Result<String> {
        self.request(job, "POST", "/api/auth/login", json!({"password":password}))?;
        self.csrf = field(
            &self.request(job, "GET", "/api/auth/csrf", Value::Null)?,
            "/token",
        )?;
        let node=field(&self.request(job,"POST","/api/provider-nodes",json!({"name":"CPA demo","type":"openai-compatible","apiType":"chat","prefix":"labcpa","baseUrl":format!("{}/v1",cpa.origin())}))?,"/node/id")?;
        let id = field(
            &self.request(
                job,
                "POST",
                "/api/providers",
                json!({"provider":node,"name":"fixture","apiKey":&cpa.auth[7..]}),
            )?,
            "/connection/id",
        )?;
        if !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("ID de conexión inválido".into());
        }
        self.request(
            job,
            "POST",
            &format!("/api/providers/{id}/sync-models?mode=import"),
            json!({}),
        )?;
        for (k, v) in [
            ("reasoning_efforts", json!("low,high")),
            ("context_length", json!(if gentle { 262144 } else { 32768 })),
            ("max_output_tokens", json!(if gentle { 4096 } else { 2048 })),
        ] {
            for target in [Some(TARGET), gentle.then_some("labcpa/lab-helper")]
                .into_iter()
                .flatten()
            {
                self.request(
                    job,
                    "PATCH",
                    "/api/model-capability-overrides",
                    json!({"target":target,"key":k,"value":v}),
                )?;
            }
        }
        self.request(job,"PATCH","/api/resilience",json!({"requestQueue":{"maxWaitMs":15000,"executionMaxWaitMs":90000,"concurrentRequests":1,"autoEnableApiKeyProviders":true}}))?;
        self.request(
            job,
            "POST",
            "/api/rate-limits",
            json!({"connectionId":id,"enabled":true}),
        )?;
        field(
            &self.request(
                job,
                "POST",
                "/api/keys",
                json!({"name":"demo","allowedConnections":[id]}),
            )?,
            "/key",
        )
    }
}
