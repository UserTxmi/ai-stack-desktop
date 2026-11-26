//! Origen efímero del WebView. Sólo sirve activos del PE fijado y rutas del SDK W1.
use super::*;
use http_body_util::{combinators::UnsyncBoxBody, BodyExt, Full, Limited};
use hyper::{
    body::{Bytes, Incoming},
    client::conn::http1,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::TokioIo;
use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

type Body = UnsyncBoxBody<Bytes, hyper::Error>;
fn reply(status: u16, mime: &str, body: impl Into<Bytes>) -> Response<Body> {
    Response::builder().status(status).header("content-type",mime).header("cache-control","no-store")
        .header("x-content-type-options","nosniff").header("referrer-policy","no-referrer")
        .header("content-security-policy", "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'sha256-jURJv6M3UYb5GqNE3+c1I0SvGlqS1+LmHVWtqFsefBk='; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' blob: data:; worker-src 'self' blob:; object-src 'none'; frame-src 'none'; frame-ancestors 'none'; form-action 'none'; base-uri 'none'")
        .body(Full::new(body.into()).map_err(|e| match e {}).boxed_unsync()).unwrap()
}
fn deny(status: u16) -> Response<Body> {
    reply(status, "application/json", "{}")
}
fn id(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
        && s.len() < 128
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn allowed(method: &str, path: &str) -> bool {
    if !path.starts_with('/') || path.contains("//") {
        return false;
    }
    let parts: Vec<_> = path.trim_start_matches('/').split('/').collect();
    match (method, parts.as_slice()) {
        ("GET", ["global", "health" | "event" | "config"])
        | (
            "GET",
            ["agent" | "config" | "provider" | "project" | "path" | "session" | "permission"
            | "question" | "event" | "command" | "mcp" | "pty" | "vcs" | "lsp" | "formatter"],
        )
        | (
            "GET",
            ["project", "current"]
            | ["provider", "auth"]
            | ["vcs", "status" | "diff"]
            | ["session", "status"]
            | ["experimental", "worktree" | "session" | "resource" | "capabilities"]
            | ["api", "reference"],
        )
        | ("POST", ["session"]) => true,
        ("GET" | "PATCH" | "DELETE", ["session", s]) => id(s, "ses_"),
        ("GET", ["session", s, "message" | "children" | "todo" | "diff"])
        | (
            "POST",
            ["session", s, "prompt_async" | "message" | "abort" | "fork" | "revert" | "unrevert" | "summarize"],
        ) => id(s, "ses_"),
        ("POST", ["session", s, "permissions", p]) => id(s, "ses_") && id(p, "per"),
        ("POST", ["permission", p, "reply"]) => id(p, "per"),
        ("POST", ["question", q, "reply" | "reject"]) => id(q, "que"),
        _ => false,
    }
}
struct Bridge {
    assets: web_assets::Assets,
    job: Arc<Job>,
    upstream: u16,
    basic: String,
    host: String,
    bootstrap: Mutex<Option<String>>,
    cookie: String,
    project: String,
}
impl Bridge {
    async fn handle(&self, req: Request<Incoming>) -> Result<Response<Body>> {
        let h = req.headers();
        let header = |k: &str| h.get(k).and_then(|v| v.to_str().ok()).unwrap_or("");
        let origin = format!("http://{}", self.host);
        if header("host") != self.host
            || (!header("origin").is_empty() && header("origin") != origin)
            || header("sec-fetch-site") == "cross-site"
        {
            return Ok(deny(403));
        }
        let path = req.uri().path();
        let method = req.method().as_str();
        if method == "GET" && path.starts_with("/open/") {
            let mut ticket = self.bootstrap.lock().safe()?;
            if ticket.as_deref() != Some(&path[6..]) {
                return Ok(deny(403));
            }
            ticket.take();
            let mut r = reply(303, "text/plain", "");
            r.headers_mut().insert(
                "location",
                format!(
                    "/{}/session",
                    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.project)
                )
                .parse()
                .safe()?,
            );
            r.headers_mut().insert(
                "set-cookie",
                format!("{}; Path=/; HttpOnly; SameSite=Strict", self.cookie)
                    .parse()
                    .safe()?,
            );
            return Ok(r);
        }
        if !header("cookie")
            .split(';')
            .any(|kv| kv.trim() == self.cookie)
        {
            return Ok(deny(401));
        }
        // Una navegación del router nunca se traduce en una petición de página a OpenCode.
        if method == "GET"
            && (header("sec-fetch-mode") == "navigate"
                || path == "/"
                || path.starts_with("/assets/")
                || path.contains('.'))
        {
            let key = if header("sec-fetch-mode") == "navigate" || path == "/" {
                "/index.html"
            } else {
                path
            };
            return Ok(match self.assets.get(key)? {
                Some((mime, bytes)) => reply(200, mime, bytes),
                None => deny(404),
            });
        }
        if !allowed(method, path) {
            return Ok(deny(404));
        }
        let target = {
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            for (k, v) in url::form_urlencoded::parse(req.uri().query().unwrap_or("").as_bytes()) {
                if k == "directory" {
                    if !v.eq_ignore_ascii_case(&self.project) {
                        return Ok(deny(403));
                    }
                } else if matches!(
                    k.as_ref(),
                    "limit"
                        | "roots"
                        | "search"
                        | "start"
                        | "cursor"
                        | "before"
                        | "after"
                        | "messageID"
                        | "mode"
                        | "context"
                ) {
                    query.append_pair(&k, &v);
                }
            }
            query.append_pair("directory", &self.project);
            format!("{path}?{}", query.finish())
        };
        let method = req.method().clone();
        let data = match Limited::new(req.into_body(), 4 << 20).collect().await {
            Ok(b) => b.to_bytes(),
            Err(_) => return Ok(deny(413)),
        };
        let socket = tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, self.upstream))
            .await
            .safe()?;
        let peer = Some(socket.local_addr().safe()?.port());
        for attempt in 0..5 {
            if self.job.owns(self.upstream, peer)? {
                break;
            }
            if attempt == 4 {
                return Err("Conexión ajena".into());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        let (mut tx, conn) = http1::handshake(TokioIo::new(socket)).await.safe()?;
        tokio::spawn(conn);
        let upstream_host = format!("127.0.0.1:{}", self.upstream);
        let request = Request::builder()
            .method(method)
            .uri(target)
            .header("host", &upstream_host)
            .header("origin", format!("http://{upstream_host}"))
            .header("authorization", &self.basic)
            .header("content-type", "application/json")
            .body(Full::new(data))
            .safe()?;
        let res = tx.send_request(request).await.safe()?;
        let status = res.status().as_u16();
        if status == 204 {
            return Ok(reply(204, "application/json", ""));
        }
        let mime = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !(mime.starts_with("application/json") || mime.starts_with("text/event-stream")) {
            return Ok(deny(502));
        }
        let mut out = reply(status, mime, "");
        if let Some(cursor) = res.headers().get("x-next-cursor") {
            out.headers_mut().insert("x-next-cursor", cursor.clone());
        }
        *out.body_mut() = res.into_body().boxed_unsync();
        Ok(out)
    }
}
pub struct Workspace {
    pub origin: String,
    pub launch: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Workspace {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
impl Supervisor {
    pub fn close_workspace(&mut self) {
        self.workspace.take();
    }
    pub fn open_workspace(&mut self, dir: &Path) -> Result<&Workspace> {
        let service = self.services.get(2).ok_or("Inicie los servicios primero")?;
        let project = dunce::canonicalize(dir).map_err(|_| "La carpeta no existe")?;
        if !project.is_dir()
            || project.parent().is_none()
            || !matches!(project.components().next(),Some(std::path::Component::Prefix(p)) if matches!(p.kind(),std::path::Prefix::Disk(_)))
            || project.starts_with(self.install.root())
            || self.install.root().starts_with(&project)
        {
            return Err("Elija una carpeta de proyecto local fuera de la instalación".into());
        }
        let assets = web_assets::Assets::open(&self.runtime.join("oc/opencode.exe"))?;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).safe()?;
        let host = listener.local_addr().safe()?.to_string();
        let origin = format!("http://{host}");
        let ticket = secret();
        let bridge = Arc::new(Bridge {
            assets,
            job: service.job.clone(),
            upstream: service.api.port,
            basic: service.api.auth.clone(),
            host,
            bootstrap: Mutex::new(Some(ticket.clone())),
            cookie: format!("w1_{}={}", &ticket[..12], secret()),
            project: s(&project),
        });
        listener.set_nonblocking(true).safe()?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .safe()?;
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let thread = std::thread::spawn(move || {
            rt.block_on(async move {
                let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                    return;
                };
                while !done.load(Ordering::Acquire) {
                    let Ok(Ok((socket, _))) =
                        tokio::time::timeout(Duration::from_millis(100), listener.accept()).await
                    else {
                        continue;
                    };
                    let b = bridge.clone();
                    tokio::spawn(async move {
                        let _ = hyper::server::conn::http1::Builder::new()
                            .serve_connection(
                                TokioIo::new(socket),
                                service_fn(move |req| {
                                    let b = b.clone();
                                    async move {
                                        Ok::<_, Infallible>(
                                            b.handle(req).await.unwrap_or_else(|_| deny(502)),
                                        )
                                    }
                                }),
                            )
                            .await;
                    });
                }
            })
        });
        self.workspace = Some(Workspace {
            launch: format!("{origin}/open/{ticket}"),
            origin,
            stop,
            thread: Some(thread),
        });
        Ok(self.workspace.as_ref().unwrap())
    }
}
