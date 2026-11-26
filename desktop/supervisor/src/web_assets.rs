//! Índice del Bun filesystem del PE exacto de W1. No ejecutar el mapa JavaScript ni
//! extraer rutas a disco. Mantener abierto el mismo archivo validado, sin compartir escritura.
use super::*;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;

pub(crate) struct Assets {
    file: Mutex<File>,
    paths: BTreeMap<String, (u64, usize)>,
}
fn bytes(file: &mut File, start: u64, size: usize) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(start)).safe()?;
    let mut buf = vec![0; size];
    file.read_exact(&mut buf).safe()?;
    Ok(buf)
}
impl Assets {
    pub fn open(path: &Path) -> Result<Self> {
        // Offsets revisados en opencode-windows-x64 1.19.6, no un parser genérico de Bun.
        const BASE: u64 = 115271176;
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(path)
            .safe()?;
        let mut hash = Sha256::new();
        io::copy(&mut file, &mut hash).safe()?;
        let pins: Value = serde_json::from_str(include_str!("../../w1.json")).unwrap();
        if pins["oc/opencode.exe"] != format!("{:x}", hash.finalize()) {
            return Err("UI: binario OpenCode no verificado".into());
        }
        let table = bytes(&mut file, BASE + 97662827, 70564)?;
        let names = bytes(&mut file, BASE + 0x5d16527, 97662827 - 0x5d16527)?;
        let mut files = BTreeMap::new();
        for row in table.chunks_exact(52) {
            let word = |n: usize| u32::from_le_bytes(row[n..n + 4].try_into().unwrap()) as usize;
            let i = word(0) - 0x5d16527;
            let key = std::str::from_utf8(&names[i..i + word(4)]).safe()?;
            files.insert(key, (BASE + word(8) as u64, word(12)));
        }
        let &(offset, size) = files
            .get("B:/~BUN/root/opencode-web-ui.gen.js")
            .ok_or("UI ausente")?;
        let script = String::from_utf8(bytes(&mut file, offset, size)?).safe()?;
        let (defs, map) = script
            .split_once("var Noe={_headers:e,")
            .ok_or("Mapa UI desconocido")?;
        let mut vars = BTreeMap::new();
        for def in defs.split("var ").skip(1) {
            let (var, path) = def.split_once("=\"").ok_or("Referencia UI inválida")?;
            vars.insert(var, files[path.trim_end_matches("\";")]);
        }
        let mut paths = BTreeMap::new();
        for entry in map
            .split_once("};export")
            .ok_or("Mapa UI incompleto")?
            .0
            .split(',')
        {
            let (key, var) = entry.split_once(':').ok_or("Entrada UI inválida")?;
            paths.insert(format!("/{}", key.trim_matches('"')), vars[var]);
        }
        if paths.len() != 952 || !paths.contains_key("/index.html") {
            return Err("UI incompleta".into());
        }
        Ok(Self {
            file: Mutex::new(file),
            paths,
        })
    }
    pub fn get(&self, path: &str) -> Result<Option<(&'static str, Vec<u8>)>> {
        let Some(&(offset, size)) = self.paths.get(path) else {
            return Ok(None);
        };
        let mime = match path.rsplit('.').next().unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "js" => "application/javascript",
            "css" => "text/css",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "ico" => "image/x-icon",
            "ttf" => "font/ttf",
            "woff2" => "font/woff2",
            "woff" => "font/woff",
            "aac" => "audio/aac",
            "mp4" => "video/mp4",
            "webmanifest" => "application/manifest+json",
            _ => return Err("Tipo UI desconocido".into()),
        };
        Ok(Some((
            mime,
            bytes(&mut *self.file.lock().safe()?, offset, size)?,
        )))
    }
}
