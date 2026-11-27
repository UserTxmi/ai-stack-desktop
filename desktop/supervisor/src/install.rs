//! Carpeta portátil exclusiva. Una instancia posee tanto la preparación como la demo.
use super::*;
use std::{
    io::{Read, Write},
    os::windows::fs::MetadataExt,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT,
};

const OWNER: &[u8] = b"AI Stack Desktop / portable / 1\n";
const DIRS: [&str; 6] = [
    "runtimes",
    "config",
    "datos",
    "cache",
    "logs",
    "preparacion",
];

pub(crate) fn disk<T>(res: io::Result<T>, action: &str) -> Result<T> {
    res.map_err(|e| {
        let reason = match (e.kind(), e.raw_os_error()) {
            (io::ErrorKind::PermissionDenied, _) => {
                "permiso denegado; use una carpeta privada con permiso de escritura"
            }
            (_, Some(32 | 33)) => {
                "archivo en uso; cierre la otra instancia o aplicación que lo usa"
            }
            (_, Some(39 | 112)) => "espacio en disco insuficiente",
            (_, Some(206)) => "ruta demasiado larga; extraiga el ZIP en una carpeta más corta",
            (io::ErrorKind::NotFound, _) => "archivo o carpeta ausente",
            _ => return format!("{action}: error de disco ({e})"),
        };
        format!("{action}: {reason}")
    })
}

pub(crate) fn is_link(m: &fs::Metadata) -> bool {
    m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

pub(crate) fn item(p: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(p) {
        Ok(m) if is_link(&m) => Err(format!(
            "No se admite un enlace o punto de redirección en {}",
            p.display()
        )),
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => disk(Err(e), "Comprobar ubicación"),
    }
}

pub(crate) fn folder(p: &Path) -> Result<()> {
    match item(p)? {
        Some(m) if m.is_dir() => Ok(()),
        None => disk(fs::create_dir(p), &format!("Crear {}", p.display())),
        _ => Err(format!("Se esperaba una carpeta en {}", p.display())),
    }
}

pub(crate) fn discard(p: &Path) -> Result<()> {
    match item(p)? {
        Some(m) if m.is_dir() => disk(fs::remove_dir_all(p), "Limpiar carpeta de preparación"),
        Some(_) => disk(fs::remove_file(p), "Limpiar archivo de preparación"),
        None => Ok(()),
    }
}

fn local_root(dir: &Path) -> Result<PathBuf> {
    use std::path::{Component, Prefix};
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumeInformationW};
    let root = disk(
        dunce::canonicalize(dir),
        "Resolver la carpeta de instalación",
    )?;
    let Some(Component::Prefix(prefix)) = root.components().next() else {
        return Err("Se necesita una carpeta de Windows".into());
    };
    let (Prefix::Disk(letter) | Prefix::VerbatimDisk(letter)) = prefix.kind() else {
        return Err("Extraiga el ZIP en un disco local NTFS".into());
    };
    let volume = [u16::from(letter), b':' as u16, b'\\' as u16, 0];
    let mut kind = [0u16; 32];
    unsafe {
        if GetDriveTypeW(volume.as_ptr()) != 3
            || GetVolumeInformationW(
                volume.as_ptr(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                kind.as_mut_ptr(),
                kind.len() as u32,
            ) == 0
        {
            return Err("No se pudo comprobar el disco local de instalación".into());
        }
    }
    if !String::from_utf16_lossy(&kind)
        .trim_end_matches('\0')
        .eq_ignore_ascii_case("NTFS")
    {
        return Err("Se necesita NTFS para las dependencias privadas de OmniRoute".into());
    }
    Ok(root)
}

pub struct Install {
    root: PathBuf,
    pub(crate) bin: PathBuf,
    _owner: File,
}

impl Install {
    pub fn beside_exe() -> Result<Self> {
        let exe = disk(std::env::current_exe(), "Ubicar ejecutable")?;
        let bin = exe.parent().ok_or("No se pudo ubicar el ejecutable")?;
        if !bin
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("bin"))
        {
            return Err("Extraiga el ZIP completo y abra el EXE desde su carpeta bin".into());
        }
        if !item(bin)?.is_some_and(|m| m.is_dir()) {
            return Err("La carpeta bin no existe".into());
        }
        let pointer = bin.join("perfil.txt");
        if item(&pointer)?.is_some() {
            let text = fs::read_to_string(pointer).safe()?;
            let state = Path::new(text.trim_start_matches('\u{feff}').trim_end());
            if !state.is_absolute() {
                return Err("Ruta de perfil instalada inválida".into());
            }
            folder(state)?;
            let mut install = Self::open(state)?;
            install.bin = bin.to_owned();
            Ok(install)
        } else {
            Self::open(bin.parent().ok_or("Falta la carpeta de instalación")?)
        }
    }

    pub(crate) fn open(root: &Path) -> Result<Self> {
        let root = local_root(root)?;
        let path = root.join(".ai-stack-desktop-portable");
        let mut owner = disk(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(&path),
            "Abrir la carpeta de instalación (otra instancia o sin permiso)",
        )?;
        if is_link(&disk(owner.metadata(), "Comprobar bloqueo de instalación")?) {
            return Err("El bloqueo de instalación no puede ser un enlace".into());
        }
        let mut mark = vec![];
        disk(
            (&mut owner)
                .take((OWNER.len() + 1) as u64)
                .read_to_end(&mut mark),
            "Leer propiedad de instalación",
        )?;
        if mark.is_empty() {
            for d in DIRS {
                if item(&root.join(d))?.is_some() {
                    return Err(format!("La carpeta {d} ya existe y no pertenece a esta instalación. Extraiga el ZIP en una carpeta nueva"));
                }
            }
            disk(
                owner.write_all(OWNER).and_then(|_| owner.sync_all()),
                "Registrar propiedad de instalación",
            )?;
        } else if mark != OWNER {
            return Err(
                "Ubicación con un marcador desconocido. Extraiga el ZIP en una carpeta nueva"
                    .into(),
            );
        }
        let install = Self {
            bin: root.join("bin"),
            root,
            _owner: owner,
        };
        install.check_layout()?;
        Ok(install)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn runtime(&self) -> PathBuf {
        self.root.join("runtimes")
    }
    pub fn webview(&self) -> Result<PathBuf> {
        let p = self.root.join("datos/webview2");
        folder(&p)?;
        Ok(p)
    }
    pub fn temporary(&self) -> PathBuf {
        self.root.join("preparacion")
    }
    pub(crate) fn check_layout(&self) -> Result<()> {
        for d in DIRS {
            folder(&self.root.join(d))?;
        }
        Ok(())
    }

    pub(crate) fn log(&self, name: &str, body: &str) {
        let logs = self.root.join("logs");
        let path = logs.join(name);
        if item(&logs).is_ok_and(|m| m.is_some_and(|m| m.is_dir())) && item(&path).is_ok() {
            let _ = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(path)
                .and_then(|mut f| f.write_all(body.as_bytes()));
        }
    }
}
