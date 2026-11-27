//! W1 reproducible: etapas pequeñas, publicación en el mismo volumen y verificación al reabrir.
use super::{
    acquire::Artifact,
    install::{discard, disk, folder, item},
    *,
};
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::{
    io::{BufReader, Read},
    path::Component,
};

const VERSIONS: &str = include_str!("../../../experiments/phase1/versions.json");
const NODE_DIR: &str = "node/node-v24.21.0-win-x64";
const NPM: &str = "node/node-v24.21.0-win-x64/node_modules/npm/bin/npm-cli.js";
const NPM_VERSION: &str = "11.19.0";
const SQLITE: &str = "package/node_modules/better-sqlite3";
const ADDON: &str = "build/Release/better_sqlite3.node";
const PEER: &str = "node_modules/@omniroute/opencode-plugin";
const COMPONENTS: [(&str, &str); 5] = [
    ("cpa", "CLIProxyAPI"),
    ("node", "Node / npm"),
    ("oc", "OpenCode"),
    ("package", "OmniRoute / SQLite"),
    ("peers", "Plugin OpenCode"),
];
const PEERS: [(&str, &str); 5] = [
    (
        "package.json",
        include_str!("../../../experiments/phase1/peers/package.json"),
    ),
    (
        "package-lock.json",
        include_str!("../../../experiments/phase1/peers/package-lock.json"),
    ),
    (
        "catalog.mjs",
        include_str!("../../../experiments/phase1/peers/catalog.mjs"),
    ),
    (
        "selection.mjs",
        include_str!("../../../experiments/phase1/peers/selection.mjs"),
    ),
    (
        "probe.mjs",
        include_str!("../../../experiments/phase1/peers/probe.mjs"),
    ),
];
const WORKSPACES: [(&str, &str); 4] = [
    (
        "package.json",
        "943c419b3d1e9e81dc87ddda4f9e1fe888fd95de908c2b7bccb13bfbdf4978e0",
    ),
    (
        "open-sse/package.json",
        "8ab2e596cfb8782ea0e20dacb4aa82babb81b7c3bed230076271227e58162817",
    ),
    (
        "packages/browser-pool/package.json",
        "6d9cc996bac5f874b941c6e2c4e7a6bf01dcb96f1d507587333373d608dcddb2",
    ),
    (
        "packages/ui-kit/package.json",
        "0c27fa221bdb35f7cb41f7c13cd50c4e6e9296a267f9afc580106f5cc635b4bd",
    ),
];

#[derive(Clone, Serialize)]
pub struct Progress {
    pub component: String,
    pub stage: String,
    pub completed: usize,
    pub stages: usize,
    pub bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Default, Serialize)]
pub struct PreparationStats {
    pub reused: usize,
    pub installed: usize,
    pub downloads: usize,
}

#[derive(Deserialize)]
struct Inventory {
    artifacts: Vec<Artifact>,
}

struct Prepare<'a, F> {
    install: &'a Install,
    stage: PathBuf,
    downloads: PathBuf,
    artifacts: Inventory,
    stats: PreparationStats,
    owns_stage: bool,
    update: F,
}

pub fn verify(install: &Install) -> Result<()> {
    let runtime = install.runtime();
    let inventory: Inventory = serde_json::from_str(VERSIONS).unwrap();
    for (id, label) in COMPONENTS {
        verify_component(&runtime, id, &inventory, true).map_err(|e| format!("{label}: {e}"))?;
    }
    Ok(())
}

pub fn prepare(install: &Install, update: impl FnMut(Progress)) -> Result<PreparationStats> {
    prepare_selected(install, update, &COMPONENTS, false)
}
const GENTLE_COMPONENTS: &[(&str, &str)] = &[
    ("cpa", "CLIProxyAPI"),
    ("node", "Node / npm"),
    ("package", "OmniRoute"),
    ("git", "MinGit"),
    ("engram", "Engram"),
    ("gpui", "Gentle Shell GPUI"),
    ("gentle", "Gentle Shell / Pi"),
];
pub fn prepare_gentle(install: &Install, update: impl FnMut(Progress)) -> Result<PreparationStats> {
    prepare_selected(install, update, GENTLE_COMPONENTS, true)
}
fn prepare_selected(
    install: &Install,
    update: impl FnMut(Progress),
    components: &[(&str, &str)],
    gentle: bool,
) -> Result<PreparationStats> {
    let tmp = install.temporary();
    let stage = tmp.join("trabajo");
    let mut p = Prepare {
        install,
        stage,
        downloads: install.root().join("cache/descargas"),
        artifacts: serde_json::from_str(VERSIONS).unwrap(),
        stats: PreparationStats::default(),
        owns_stage: false,
        update,
    };
    if gentle {
        let extra: Inventory =
            serde_json::from_str(include_str!("../../gentle-assets.json")).unwrap();
        p.artifacts.artifacts.extend(extra.artifacts);
    }
    let result = p.run(components);
    let clean = if p.owns_stage {
        discard(&p.stage)
    } else {
        Ok(())
    };
    match result.and(clean) {
        Ok(()) => {
            install.log(
                "preparacion.txt",
                &format!(
                    "W1 íntegro; reutilizados={}, instalados={}, descargas={}\n",
                    p.stats.reused, p.stats.installed, p.stats.downloads
                ),
            );
            Ok(p.stats)
        }
        Err(e) => {
            install.log("preparacion.txt", &e);
            Err(e)
        }
    }
}

impl<F: FnMut(Progress)> Prepare<'_, F> {
    fn report(&mut self, id: &str, stage: &str, bytes: u64, total_bytes: Option<u64>) {
        (self.update)(Progress {
            component: id.into(),
            stage: stage.into(),
            bytes,
            total_bytes,
            completed: self.stats.reused + self.stats.installed,
            stages: COMPONENTS.len(),
        });
    }

    fn run(&mut self, components: &[(&str, &str)]) -> Result<()> {
        self.report(
            "Instalación",
            "Comprobando carpetas y recuperando preparación anterior",
            0,
            None,
        );
        self.install.check_layout()?;
        discard(&self.stage)?;
        folder(&self.stage)?;
        self.owns_stage = true;
        folder(&self.downloads)?;
        let runtime = self.install.runtime();
        for &(id, label) in components {
            self.report(label, "Verificando archivos instalados", 0, None);
            match verify_component(&runtime, id, &self.artifacts, true) {
                Ok(()) => {
                    self.stats.reused += 1;
                    self.report(label, "Íntegro · reutilizado", 0, None);
                    continue;
                }
                Err(e) => self.report(label, &format!("Se requiere preparación: {e}"), 0, None),
            }
            self.component(&runtime, id, label)
                .map_err(|e| format!("{label}: {e}"))?;
            self.stats.installed += 1;
            self.report(label, "Instalado y verificado", 0, None);
        }
        self.report(
            "W1",
            "Comprobando enlaces privados y SQLite con una consulta de prueba",
            0,
            None,
        );
        verify_workspace_links(&runtime)?;
        let env = self.build_env(&runtime.join(NODE))?;
        self.sqlite(&runtime, &env)?;
        self.report("W1", "Verificación completada", 0, None);
        Ok(())
    }

    fn fetch(&mut self, id: &str, label: &str) -> Result<PathBuf> {
        let art = artifact(&self.artifacts, id);
        let count = self.stats.reused + self.stats.installed;
        let display = format!("{label} · {id} {}", art.version);
        let mut fetched = false;
        let update = &mut self.update;
        let file = art
            .acquire(&self.downloads, &mut |status, bytes, total_bytes| {
                fetched |= status == "Descargando";
                update(Progress {
                    component: display.clone(),
                    stage: status.into(),
                    bytes,
                    total_bytes,
                    completed: count,
                    stages: COMPONENTS.len(),
                });
            })
            .map_err(|e| format!("{display}: {e}"))?;
        self.stats.downloads += usize::from(fetched);
        Ok(file)
    }

    fn component(&mut self, runtime: &Path, id: &str, label: &str) -> Result<()> {
        // No sustituir una carpeta válida antes de terminar y verificar su posible reemplazo.
        let output = runtime.join(id);
        item(&output)?;
        let dir = self.stage.join(id);
        match id {
            "package" => self.gateway(runtime)?,
            "peers" => self.peers(runtime)?,
            "gentle" => self.gentle(runtime)?,
            _ => {
                let archive = self.fetch(id, label)?;
                self.report(label, "Extrayendo archivo verificado", 0, None);
                folder(&dir)?;
                unzip(&archive, &dir)?;
            }
        }
        self.report(label, "Verificando huellas y versiones W1", 0, None);
        verify_component(&self.stage, id, &self.artifacts, false)?;
        if id == "package" {
            self.rebase_workspaces(runtime)?;
        }
        let old = self.stage.join("componente-anterior");
        if item(&output)?.is_some() {
            disk(fs::rename(&output, &old), "Aislar componente incompleto")?;
        }
        publish(&dir, &output, &old)?;
        discard(&old)?;
        Ok(())
    }

    fn gateway(&mut self, runtime: &Path) -> Result<()> {
        let omni = self.fetch("omni", "OmniRoute")?;
        let lock = self.fetch("lock", "OmniRoute")?;
        let workspaces = self.fetch("workspaces", "OmniRoute")?;
        let expected = artifact(&self.artifacts, "sqlite");
        check_lock(&lock, "node_modules/better-sqlite3", expected)?;
        self.report(
            "OmniRoute",
            "Extrayendo distribución y workspaces fijados",
            0,
            None,
        );
        untar(&omni, &self.stage, |path| {
            path.starts_with("package/").then_some(path)
        })?;
        let package = self.stage.join("package");
        untar(&workspaces, &package, |path| {
            let (_, rel) = path.split_once('/')?;
            (WORKSPACES.iter().any(|(name, _)| rel == *name)
                || rel.starts_with("packages/browser-pool/")
                || rel.starts_with("packages/ui-kit/"))
            .then_some(rel)
        })?;
        for (path, hash) in WORKSPACES {
            check_hash(&package, path, hash)?;
        }
        disk(
            fs::copy(lock, package.join("package-lock.json")),
            "Copiar lock de OmniRoute",
        )?;
        backport(&self.stage)?;
        let env = self.build_env(&runtime.join(NODE))?;
        self.report(
            "OmniRoute",
            "npm ci · instalando dependencias privadas (sin scripts)",
            0,
            None,
        );
        self.npm(runtime, &package, &env, true)?;
        self.report("SQLite", "Comprobando el binario nativo y su ABI", 0, None);
        // npm ci verifica el SRI del tarball contra el lock; se verifica además el
        // binario extraído antes de cargarlo, sin descargar dos veces el mismo paquete.
        let prebuild = format!("{SQLITE}/prebuilds/win32-x64.node");
        check_hash(&self.stage, &prebuild, &w1()[&format!("{SQLITE}/{ADDON}")])?;
        for dest in [
            package.join("dist/node_modules/better-sqlite3"),
            self.stage.join(SQLITE),
        ] {
            if item(&dest)?.is_some_and(|m| m.is_dir()) {
                let target = dest.join(ADDON);
                disk(
                    fs::create_dir_all(target.parent().unwrap()),
                    "Crear destino de SQLite",
                )?;
                disk(
                    fs::copy(self.stage.join(&prebuild), target),
                    "Preparar SQLite nativo",
                )?;
            }
        }
        self.sqlite(&self.stage, &env)?;
        Ok(())
    }

    fn gentle(&mut self, runtime: &Path) -> Result<()> {
        let dir = self.stage.join("gentle");
        folder(&dir)?;
        for (name, data) in [
            (
                "package.json",
                include_str!("../../../experiments/gentle/package.json"),
            ),
            (
                "package-lock.json",
                include_str!("../../../experiments/gentle/package-lock.json"),
            ),
        ] {
            fs::write(dir.join(name), data).safe()?;
        }
        let mut env = self.build_env(&runtime.join(NODE))?;
        self.npm(runtime, &dir, &env, false)?;
        let pkg = dir.join("node_modules/gentle-pi");
        let patch: Value =
            serde_json::from_str(include_str!("../../../experiments/gentle/consent.json")).unwrap();
        let mut args = vec!["-c".into(), "core.autocrlf=false".into(), "apply".into()];
        for path in patch["files"].as_object().unwrap().keys() {
            args.push(format!("--include={path}"));
        }
        args.push(s(&self.fetch("consent", "Consentimiento Gentle Shell")?));
        env.insert("GIT_CEILING_DIRECTORIES".into(), s(&dir));
        run(
            &runtime.join("git/cmd/git.exe"),
            args,
            &pkg,
            &env,
            Duration::from_secs(30),
            "Consentimiento",
        )?;
        let src = self.install.root().join("bin/gentle-ai.exe");
        let bundle = pkg.join(".gentle-ai/v5.0.0");
        fs::create_dir_all(&bundle).safe()?;
        fs::copy(src, bundle.join("gentle-ai.exe")).safe()?;
        fs::copy(
            self.install.root().join("bin/integrity.json"),
            bundle.join("integrity.json"),
        )
        .safe()?;
        Ok(())
    }

    fn peers(&mut self, runtime: &Path) -> Result<()> {
        let dir = self.stage.join("peers");
        folder(&dir)?;
        for (name, content) in PEERS {
            disk(
                fs::write(dir.join(name), content),
                "Copiar recursos del plugin",
            )?;
        }
        check_lock(
            &dir.join("package-lock.json"),
            PEER,
            artifact(&self.artifacts, "plugin"),
        )?;
        self.report(
            "Plugin OpenCode",
            "npm ci · instalando plugin y dependencias privadas",
            0,
            None,
        );
        self.npm(runtime, &dir, &self.build_env(&runtime.join(NODE))?, false)
    }

    fn build_env(&self, node: &Path) -> Result<Env> {
        let home = self.stage.join("perfil");
        let mut env = environment(&home, node)?;
        let cache = self.install.root().join("cache/npm");
        let logs = self.install.root().join("logs/npm");
        folder(&cache)?;
        folder(&logs)?;
        for (k, v) in [
            ("cache", s(&cache)),
            ("logs-dir", s(&logs)),
            ("logs-max", "5".into()),
            ("userconfig", s(&home.join("empty-user.npmrc"))),
            ("globalconfig", s(&home.join("empty-global.npmrc"))),
            ("registry", "https://registry.npmjs.org".into()),
            ("update-notifier", "false".into()),
            ("progress", "false".into()),
            ("fetch-retries", "1".into()),
            ("fetch-timeout", "90000".into()),
        ] {
            env.insert(format!("npm_config_{k}").replace('-', "_"), v);
        }
        Ok(env)
    }

    fn npm(&self, runtime: &Path, cwd: &Path, env: &Env, legacy: bool) -> Result<()> {
        let mut args = vec![s(&runtime.join(NPM))];
        args.extend(
            [
                "ci",
                "--omit=dev",
                "--include=optional",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
                "--prefer-offline",
            ]
            .map(str::to_owned),
        );
        if legacy {
            args.push("--legacy-peer-deps".into());
        }
        run(
            &runtime.join(NODE),
            args,
            cwd,
            env,
            Duration::from_secs(900),
            "npm ci",
        )
        .map(|_| ())
    }

    fn sqlite(&self, tree: &Path, env: &Env) -> Result<()> {
        let code = "const db=require('better-sqlite3')(':memory:'); if(db.prepare('select 41+1 as n').get().n!==42) process.exit(1); db.close(); console.log(process.version, process.arch, process.versions.modules)";
        let output = run(
            &self.install.runtime().join(NODE),
            vec!["-e".into(), code.into()],
            &tree.join("package/dist"),
            env,
            Duration::from_secs(30),
            "SQLite nativo",
        )?;
        if output.trim() != format!("v{} x64 137", artifact(&self.artifacts, "node").version) {
            return Err("SQLite: no se confirmó la versión y ABI x64 de Node".into());
        }
        Ok(())
    }

    fn rebase_workspaces(&self, runtime: &Path) -> Result<()> {
        // npm usa junctions NTFS absolutas en Windows. La publicación mueve el árbol,
        // por eso deben apuntar al destino definitivo antes del rename.
        let code = r#"const fs=require('node:fs'), path=require('node:path');
for(const [name, target] of JSON.parse(process.argv[2])) {
  const link=path.join('node_modules/@omniroute',name);
  if(!fs.lstatSync(link).isSymbolicLink() || fs.realpathSync(link)!==fs.realpathSync(target)) process.exit(2);
  fs.unlinkSync(link);
  fs.symlinkSync(path.join(process.argv[1],target),link,'junction');
}"#;
        run(
            &runtime.join(NODE),
            vec![
                "-e".into(),
                code.into(),
                s(&runtime.join("package")),
                serde_json::to_string(&LINKS).unwrap(),
            ],
            &self.stage.join("package"),
            &self.build_env(&runtime.join(NODE))?,
            Duration::from_secs(30),
            "Enlaces privados de OmniRoute",
        )?;
        Ok(())
    }
}

fn publish(dir: &Path, output: &Path, old: &Path) -> Result<()> {
    let r = disk(fs::rename(dir, output), "Publicar componente verificado");
    if r.is_err() && old.exists() {
        let _ = fs::rename(old, output);
    }
    r
}

fn run(
    exe: &Path,
    args: Vec<String>,
    cwd: &Path,
    env: &Env,
    timeout: Duration,
    label: &str,
) -> Result<String> {
    let err = |e: String| format!("{label}: {e}");
    let mut job = Job::captured(
        Spec {
            exe: exe.to_owned(),
            args,
            input: String::new(),
        },
        cwd,
        env,
    )
    .map_err(err)?;
    let (ok, output) = job.wait(timeout).map_err(err)?;
    if ok {
        return Ok(output);
    }
    let codes = [
        "EACCES",
        "EPERM",
        "ENOSPC",
        "EINTEGRITY",
        "ECONNRESET",
        "ETIMEDOUT",
        "ENOTFOUND",
        "EAI_AGAIN",
        "CERT_HAS_EXPIRED",
        "UNABLE_TO_VERIFY_LEAF_SIGNATURE",
        "ERR_DLOPEN_FAILED",
        "MODULE_NOT_FOUND",
    ];
    let reason = match codes.into_iter().find(|c| output.contains(c)) {
        Some("EACCES" | "EPERM") => "permiso denegado o archivo bloqueado",
        Some("ENOSPC") => "espacio en disco insuficiente",
        Some("EINTEGRITY") => "integridad incorrecta de una dependencia",
        Some("ERR_DLOPEN_FAILED") => "no se pudo cargar el binario nativo (ABI o DLL de Windows)",
        Some("MODULE_NOT_FOUND") => "dependencia privada ausente",
        Some(_) => "error de red o de certificado; compruebe acceso a registry.npmjs.org y vuelva a intentar",
        None => "el proceso terminó con error; consulte logs/npm si falló npm",
    };
    Err(format!("{label}: {reason}"))
}

fn artifact<'a>(inventory: &'a Inventory, id: &str) -> &'a Artifact {
    inventory.artifacts.iter().find(|a| a.id == id).unwrap()
}
fn json_file(path: &Path) -> Result<Value> {
    let data = disk(fs::read(path), "Leer manifiesto")?;
    serde_json::from_slice(&data).map_err(|_| "Manifiesto JSON dañado".into())
}
fn version(path: &Path, expected: &str) -> Result<()> {
    if json_file(path)?["version"] == expected {
        Ok(())
    } else {
        Err(format!("No coincide la versión fijada {expected}"))
    }
}
fn check_lock(path: &Path, key: &str, expected: &Artifact) -> Result<()> {
    let lock = json_file(path)?;
    let locked = &lock["packages"][key];
    if locked["version"] != expected.version
        || locked["resolved"] != expected.url
        || locked["integrity"].as_str() != expected.sri.as_deref()
    {
        Err(format!("{} no coincide con el lock fijado", expected.id))
    } else {
        Ok(())
    }
}

pub(super) fn digest(path: &Path) -> Result<String> {
    let mut h = Sha256::new();
    let mut f = disk(File::open(path), &format!("Leer {}", path.display()))?;
    disk(io::copy(&mut f, &mut h), "Calcular SHA-256")?;
    Ok(format!("{:x}", h.finalize()))
}

fn check_regular(root: &Path, name: &str) -> Result<PathBuf> {
    // npm crea junctions para los tres workspaces. Ninguna ruta verificada debe pasar por uno.
    let mut p = root.to_owned();
    for part in Path::new(name).iter() {
        p.push(part);
        if item(&p)?.is_none() {
            return Err(format!("Falta {name}"));
        }
    }
    Ok(p)
}

fn check_hash(root: &Path, name: &str, hash: &str) -> Result<()> {
    if digest(&check_regular(root, name)?)? != hash {
        return Err(format!("SHA-256 incorrecto: {name}"));
    }
    Ok(())
}

fn w1() -> BTreeMap<String, String> {
    serde_json::from_str(include_str!("../../w1.json")).unwrap()
}
const LINKS: [(&str, &str); 3] = [
    ("browser-pool", "packages/browser-pool"),
    ("open-sse", "open-sse"),
    ("ui-kit", "packages/ui-kit"),
];
fn verify_workspace_links(root: &Path) -> Result<()> {
    let pkg = root.join("package");
    for (name, target) in LINKS {
        let link = pkg.join("node_modules/@omniroute").join(name);
        let actual = disk(
            dunce::canonicalize(&link),
            "Resolver enlace privado de OmniRoute",
        )?;
        let expected = disk(
            dunce::canonicalize(pkg.join(target)),
            "Resolver destino de workspace",
        )?;
        if actual != expected || !expected.starts_with(&pkg) {
            return Err(format!(
                "Enlace de workspace fuera de la instalación: {name}"
            ));
        }
    }
    Ok(())
}
fn verify_component(root: &Path, id: &str, inventory: &Inventory, installed: bool) -> Result<()> {
    item(root)?;
    for (name, hash) in w1()
        .into_iter()
        .filter(|(name, _)| name.split('/').next() == Some(id))
    {
        check_hash(root, &name, &hash)?;
    }
    match id {
        "git" => {
            check_hash(
                root,
                "git/cmd/git.exe",
                "177bc8a9af84fa3a400c694c975fda731d048279c074e1a93947ce8b067ebb48",
            )?;
            check_hash(
                root,
                "git/usr/bin/sh.exe",
                "5490d0da5e7cf9d92068cc48fcc590f2bcf8564add8ff91c3b5fe541eb2d72e3",
            )?;
        }
        "engram" => check_hash(
            root,
            "engram/engram.exe",
            "207a1a56d0a277ca4f84427130c5aafbdeddd0f698b246736b54cde9b3c08098",
        )?,
        "gpui" => check_hash(
            root,
            "gpui/gentle-shell-gpui.exe",
            "3fb8fa1e7b50374d2ced7388b85785709a542fe2180c982e3cab6858fe41cf2f",
        )?,
        "gentle" => {
            let patch: Value =
                serde_json::from_str(include_str!("../../../experiments/gentle/consent.json"))
                    .unwrap();
            for (p, h) in patch["files"].as_object().unwrap() {
                check_hash(
                    root,
                    &format!("gentle/node_modules/gentle-pi/{p}"),
                    h.as_str().unwrap(),
                )?;
            }
            for (p, h) in [
                (
                    "gentle-pi/bin/gentle-shell.mjs",
                    "e4d257c1cbe67005e87edf49379c5e50708dab2e8389f565e0f8cb57fec9bb60",
                ),
                (
                    "@earendil-works/pi-coding-agent/dist/bundle/cli.js",
                    "e79626f2dd6f94aa45d30f3fa63cd84319a6eefcd150b353cfaf274366926774",
                ),
                (
                    "gentle-engram/index.ts",
                    "78afeea4a150441854cd8ce49fbf043a3ee117dc6a0d8ce8519c66a7cfe5833c",
                ),
            ] {
                check_hash(root, &format!("gentle/node_modules/{p}"), h)?;
            }
            let rel = "gentle/node_modules/gentle-pi/.gentle-ai/v5.0.0";
            let manifest = json_file(&check_regular(root, &format!("{rel}/integrity.json"))?)?;
            check_hash(
                root,
                &format!("{rel}/gentle-ai.exe"),
                manifest["binarySha256"]
                    .as_str()
                    .ok_or("Falta huella de Gentle AI")?,
            )?;
        }
        "node" => {
            check_hash(
                root,
                NPM,
                "3ce7cba6f5128dd5f54c98b6a5036b0f850496878cc2e21044b675fe3c594e3e",
            )?;
            check_hash(
                root,
                &format!("{NODE_DIR}/node_modules/npm/lib/cli.js"),
                "d97a2b1a2170d6094625956d6ad5ba5d5edfce26bc3787c51447c9737b6afc2d",
            )?;
            let p = format!("{NODE_DIR}/node_modules/npm/package.json");
            check_hash(
                root,
                &p,
                "44ff936daa5cd7f9f99120d5db7c2e5e21283125433e61d1b42912aefe8b1bc6",
            )?;
            version(&root.join(p), NPM_VERSION)?;
        }
        "package" => {
            let lock = artifact(inventory, "lock");
            check_hash(
                root,
                "package/package-lock.json",
                lock.sha256.as_ref().unwrap(),
            )?;
            for (rel, hash) in WORKSPACES {
                check_hash(root, &format!("package/{rel}"), hash)?;
            }
            version(
                &root.join("package/package.json"),
                &artifact(inventory, "omni").version,
            )?;
            check_regular(root, &format!("{SQLITE}/package.json"))?;
            version(
                &root.join(SQLITE).join("package.json"),
                &artifact(inventory, "sqlite").version,
            )?;
            if installed {
                verify_workspace_links(root)?;
            }
        }
        "peers" => {
            for (name, data) in PEERS {
                check_hash(
                    root,
                    &format!("peers/{name}"),
                    &format!("{:x}", Sha256::digest(data)),
                )?;
            }
            check_regular(root, &format!("peers/{PEER}/package.json"))?;
            version(
                &root.join("peers").join(PEER).join("package.json"),
                &artifact(inventory, "plugin").version,
            )?;
        }
        // Las distribuciones Windows de CPA y OpenCode se identifican por el SHA-256
        // del EXE fijado por el inventario de W1, además del hash de su ZIP versionado.
        _ => (),
    }
    Ok(())
}

#[derive(Deserialize)]
struct Backport {
    chunks: BTreeMap<String, (String, usize)>,
    graphs: Vec<Vec<(String, String)>>,
}
fn backport(root: &Path) -> Result<()> {
    let patch: Backport = serde_json::from_str(include_str!("../../w1-backport.json")).unwrap();
    let hashes = w1();
    let mut staged = Vec::new();
    for (name, (hash, graph)) in patch.chunks {
        let rel = format!("package/dist/.build/next/server/chunks/{name}");
        check_hash(root, &rel, &hash)?;
        let path = root.join(&rel);
        let mut text = disk(fs::read_to_string(&path), "Leer chunk W1")?;
        for (anchor, replacement) in &patch.graphs[graph] {
            if text.matches(anchor).count() != 1 {
                return Err("Ancla del parche W1 inesperada".into());
            }
            text = text.replacen(anchor, replacement, 1);
        }
        if format!("{:x}", Sha256::digest(text.as_bytes())) != hashes[&rel] {
            return Err("El resultado del parche no coincide con W1".into());
        }
        staged.push((path, text));
    }
    for (path, text) in staged {
        disk(fs::write(path, text), "Aplicar parche W1")?;
    }
    Ok(())
}

fn archive_path(path: &str) -> Result<&Path> {
    let path = path.trim_end_matches('/');
    let p = Path::new(path);
    if path.is_empty()
        || path.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || s.ends_with(['.', ' '])
                || s.chars()
                    .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
                || matches!(
                    s.split('.').next().unwrap().to_ascii_uppercase().as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
        })
        || p.components().any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Ruta insegura en archivo de distribución".into());
    }
    Ok(p)
}

fn write_entry(dest: &Path, read: &mut impl Read) -> Result<()> {
    disk(
        fs::create_dir_all(dest.parent().unwrap()),
        "Crear árbol de extracción",
    )?;
    let mut f = disk(File::create_new(dest), "Crear archivo extraído")?;
    disk(io::copy(read, &mut f), "Extraer datos")?;
    Ok(())
}
fn unzip(source: &Path, dir: &Path) -> Result<()> {
    let file = disk(File::open(source), "Abrir ZIP verificado")?;
    let mut zip =
        zip::ZipArchive::new(BufReader::new(file)).map_err(|_| "ZIP de distribución inválido")?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|_| "Entrada ZIP inválida")?;
        let dest = dir.join(archive_path(entry.name())?);
        if entry.is_symlink() {
            return Err("El ZIP contiene un enlace no permitido".into());
        }
        if entry.is_dir() {
            disk(fs::create_dir_all(dest), "Crear carpeta ZIP")?;
        } else {
            write_entry(&dest, &mut entry)?;
        }
    }
    Ok(())
}
fn untar(source: &Path, dir: &Path, select: impl Fn(&str) -> Option<&str>) -> Result<()> {
    let file = disk(File::open(source), "Abrir tarball verificado")?;
    let mut tar = tar::Archive::new(GzDecoder::new(BufReader::new(file)));
    for entry in disk(tar.entries(), "Leer tarball")? {
        let mut entry = disk(entry, "Leer entrada tar")?;
        let path = disk(entry.path(), "Leer ruta tar")?;
        let Some(path) = select(path.to_str().ok_or("Ruta TAR no válida")?) else {
            continue;
        };
        let dest = dir.join(archive_path(path)?);
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            disk(fs::create_dir_all(dest), "Crear carpeta TAR")?;
        } else if kind.is_file() {
            // El manifiesto de workspace puede reemplazar el distribuido, nunca otros destinos.
            if dest.is_file() && WORKSPACES.iter().any(|(n, _)| dir.join(n) == dest) {
                disk(fs::remove_file(&dest), "Restaurar manifiesto original")?;
            }
            write_entry(&dest, &mut entry)?;
        } else {
            return Err("El TAR contiene un enlace o dispositivo no permitido".into());
        }
    }
    Ok(())
}
