//! Descargas HTTPS de artefactos fijados. Un archivo .part nunca es una entrada del instalador.
use super::{
    install::{discard, disk, item},
    *,
};
use std::{
    io::Write,
    ptr::{null, null_mut},
};
use windows_sys::Win32::Networking::WinHttp::*;

pub(crate) const MAX_DOWNLOAD: u64 = 1 << 30;

#[derive(serde::Deserialize)]
pub(crate) struct Artifact {
    pub id: String,
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
    pub sri: Option<String>,
}

impl Artifact {
    pub fn verify(&self, path: &Path) -> Result<bool> {
        let Some(meta) = item(path)? else {
            return Ok(false);
        };
        if !meta.is_file() || meta.len() > MAX_DOWNLOAD {
            return Ok(false);
        }
        let mut file = disk(File::open(path), "Leer descarga")?;
        if let Some(expected) = &self.sha256 {
            let mut h = Sha256::new();
            disk(io::copy(&mut file, &mut h), "Comprobar SHA-256")?;
            Ok(format!("{:x}", h.finalize()) == *expected)
        } else {
            let mut h = sha2::Sha512::new();
            disk(io::copy(&mut file, &mut h), "Comprobar SRI SHA-512")?;
            let actual = format!(
                "sha512-{}",
                base64::engine::general_purpose::STANDARD.encode(h.finalize())
            );
            Ok(self.sri.as_deref() == Some(&actual))
        }
    }

    pub fn acquire(
        &self,
        downloads: &Path,
        update: &mut impl FnMut(&str, u64, Option<u64>),
    ) -> Result<PathBuf> {
        self.acquire_with(downloads, update, |out, update| {
            download(&self.url, out, update)
        })
    }

    fn acquire_with(
        &self,
        downloads: &Path,
        update: &mut impl FnMut(&str, u64, Option<u64>),
        transfer: impl FnOnce(&mut File, &mut dyn FnMut(u64, Option<u64>)) -> Result<()>,
    ) -> Result<PathBuf> {
        let leaf = self.url.rsplit('/').next().unwrap();
        let path = downloads.join(leaf);
        let part = downloads.join(format!("{leaf}.part"));
        if let Some(meta) = item(&part)? {
            if !meta.is_file() {
                return Err("La ruta de descarga parcial no es un archivo".into());
            }
            discard(&part)?;
        }
        update("Verificando archivo en caché", 0, None);
        if self.verify(&path)? {
            return Ok(path);
        }
        if let Some(m) = item(&path)? {
            if !m.is_file() {
                return Err("La ruta de descarga no es un archivo".into());
            }
            disk(fs::remove_file(&path), "Retirar la descarga corrupta")?;
            update("Archivo corrupto: se descargará de nuevo", 0, None);
        }
        let result = (|| {
            let mut out = disk(File::create_new(&part), "Crear descarga temporal")?;
            update("Descargando", 0, None);
            transfer(&mut out, &mut |n, total| update("Descargando", n, total))?;
            disk(out.sync_all(), "Guardar descarga temporal")?;
            drop(out);
            update("Verificando integridad de descarga", 0, None);
            if !self.verify(&part)? {
                return Err("El hash de la descarga no coincide; el archivo se descartó".into());
            }
            disk(fs::rename(&part, &path), "Confirmar descarga verificada")
        })();
        if result.is_err() {
            let _ = discard(&part);
        }
        result.map(|()| path)
    }
}

struct Http(*mut core::ffi::c_void);
impl Http {
    fn new(h: *mut core::ffi::c_void) -> Result<Self> {
        if h.is_null() {
            Err(net_error())
        } else {
            Ok(Self(h))
        }
    }
    fn option(&self, key: u32, value: u32) -> Result<()> {
        ok(unsafe { WinHttpSetOption(self.0, key, (&value as *const u32).cast(), 4) })
    }
    fn header(&self, kind: u32) -> Result<Option<u64>> {
        // La API NUMBER devuelve un DWORD; las descargas fijadas son inferiores a 1 GiB.
        let mut data = 0u32;
        let mut size = 4;
        let code = unsafe {
            WinHttpQueryHeaders(
                self.0,
                kind | WINHTTP_QUERY_FLAG_NUMBER,
                null(),
                (&mut data as *mut u32).cast(),
                &mut size,
                null_mut(),
            )
        };
        if code != 0 {
            return Ok(Some(data.into()));
        }
        if io::Error::last_os_error().raw_os_error() == Some(ERROR_WINHTTP_HEADER_NOT_FOUND as i32)
        {
            return Ok(None);
        }
        Err(net_error())
    }
}
impl Drop for Http {
    fn drop(&mut self) {
        unsafe {
            WinHttpCloseHandle(self.0);
        }
    }
}
fn net_error() -> String {
    format!("No se pudo completar la descarga HTTPS (WinHTTP {}). Compruebe la conexión, el proxy y el reloj, y vuelva a intentar",
        io::Error::last_os_error().raw_os_error().unwrap_or(0))
}
fn ok(code: i32) -> Result<()> {
    if code == 0 {
        Err(net_error())
    } else {
        Ok(())
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

fn download(url: &str, out: &mut File, update: &mut dyn FnMut(u64, Option<u64>)) -> Result<()> {
    let u = url::Url::parse(url).safe()?;
    if u.scheme() != "https"
        || u.port().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("Origen de descarga inesperado".into());
    }
    let host = wide(u.host_str().ok_or("Falta el servidor de descarga")?);
    let path = wide(u.path());
    let agent = wide(concat!("AI-Stack-Desktop/", env!("CARGO_PKG_VERSION")));
    let sess = Http::new(unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_NO_PROXY,
            null(),
            null(),
            0,
        )
    })?;
    ok(unsafe { WinHttpSetTimeouts(sess.0, 20_000, 20_000, 30_000, 45_000) })?;
    let conn = Http::new(unsafe { WinHttpConnect(sess.0, host.as_ptr(), 443, 0) })?;
    let req = Http::new(unsafe {
        WinHttpOpenRequest(
            conn.0,
            null(),
            path.as_ptr(),
            null(),
            null(),
            null(),
            WINHTTP_FLAG_SECURE,
        )
    })?;
    req.option(
        WINHTTP_OPTION_REDIRECT_POLICY,
        WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP,
    )?;
    req.option(WINHTTP_OPTION_MAX_HTTP_AUTOMATIC_REDIRECTS, 5)?;
    req.option(
        WINHTTP_OPTION_DISABLE_FEATURE,
        WINHTTP_DISABLE_AUTHENTICATION | WINHTTP_DISABLE_COOKIES,
    )?;
    ok(unsafe { WinHttpSendRequest(req.0, null(), 0, null(), 0, 0, 0) })?;
    ok(unsafe { WinHttpReceiveResponse(req.0, null_mut()) })?;
    let status = req.header(WINHTTP_QUERY_STATUS_CODE)?.unwrap_or(0);
    if status != 200 {
        return Err(format!("El servidor de descarga respondió HTTP {status}"));
    }
    let total = req.header(WINHTTP_QUERY_CONTENT_LENGTH)?;
    transfer_body(out, total, update, |buf| {
        let mut count = 0;
        ok(unsafe {
            WinHttpReadData(req.0, buf.as_mut_ptr().cast(), buf.len() as u32, &mut count)
        })?;
        Ok(count as usize)
    })
}

fn transfer_body(
    out: &mut impl Write,
    total: Option<u64>,
    update: &mut dyn FnMut(u64, Option<u64>),
    mut read: impl FnMut(&mut [u8]) -> Result<usize>,
) -> Result<()> {
    if total.is_some_and(|n| n > MAX_DOWNLOAD) {
        return Err("Descarga demasiado grande".into());
    }
    let mut received = 0;
    let mut tick = Instant::now();
    let until = tick + Duration::from_secs(20 * 60);
    update(0, total);
    let mut buf = [0; 64 * 1024];
    loop {
        if Instant::now() > until {
            return Err("Se agotó el plazo total de descarga".into());
        }
        let n = read(&mut buf)?;
        if n == 0 {
            break;
        }
        received += n as u64;
        if received > total.unwrap_or(MAX_DOWNLOAD) {
            return Err("Descarga más larga de lo declarado".into());
        }
        disk(out.write_all(&buf[..n]), "Escribir descarga")?;
        if tick.elapsed() >= Duration::from_millis(200) {
            update(received, total);
            tick = Instant::now();
        }
    }
    if total.is_some_and(|len| received != len) {
        return Err("Descarga interrumpida; vuelva a intentar".into());
    }
    update(received, total);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    #[test]
    fn descarga_cortada_corrupta_y_recuperacion_sin_repetir() {
        let tmp = std::env::temp_dir().join(format!("W1 descarga {}", secret()));
        fs::create_dir(&tmp).unwrap();
        let _tmp = crate::tests::Remove(tmp.clone());
        let bytes = b"artefacto fijado";
        let artifact = Artifact {
            id: "w1".into(),
            version: "1".into(),
            url: "https://example.invalid/w1.zip".into(),
            sha256: Some(format!("{:x}", Sha256::digest(bytes))),
            sri: None,
        };
        let mut events = Vec::new();
        let mut update = |label: &str, n, len| events.push((label.to_owned(), n, len));
        let result = artifact.acquire_with(&tmp, &mut update, |out, progress| {
            let mut source = Cursor::new(&bytes[..4]);
            transfer_body(out, Some(bytes.len() as u64), progress, |buf| {
                source.read(buf).safe()
            })
        });
        assert!(result.unwrap_err().contains("interrumpida"));
        assert!(!tmp.join("w1.zip").exists());
        assert!(!tmp.join("w1.zip.part").exists());

        fs::write(tmp.join("w1.zip.part"), b"resto de cierre forzado").unwrap();
        fs::write(tmp.join("w1.zip"), "caché corrupta").unwrap();
        assert!(artifact
            .acquire_with(&tmp, &mut update, |out, _| out
                .write_all(b"otra descarga corrupta")
                .safe())
            .unwrap_err()
            .contains("hash"));
        assert!(!tmp.join("w1.zip").exists());
        artifact
            .acquire_with(&tmp, &mut update, |out, progress| {
                let mut source = Cursor::new(bytes);
                transfer_body(out, Some(bytes.len() as u64), progress, |buf| {
                    source.read(buf).safe()
                })
            })
            .unwrap();
        artifact
            .acquire_with(&tmp, &mut update, |_, _| {
                panic!("no volver a obtener el artefacto válido")
            })
            .unwrap();
        assert!(events.iter().any(|(s, n, total)| s == "Descargando"
            && *n == bytes.len() as u64
            && *total == Some(*n)));
        assert!(events
            .iter()
            .any(|(s, _, _)| s.contains("Archivo corrupto")));
    }
}
