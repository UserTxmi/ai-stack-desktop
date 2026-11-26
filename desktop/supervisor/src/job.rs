//! Handles exclusivos. El trabajador no recibe argv hasta quedar dentro del Job.
use super::*;
use std::{
    io::{Read, Write},
    mem::{size_of, size_of_val, zeroed},
    os::windows::{
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
        process::CommandExt,
    },
    process::{Child, Command, Stdio},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use windows_sys::Win32::{NetworkManagement::IpHelper::GetExtendedTcpTable, System::JobObjects::*};

#[derive(Serialize, serde::Deserialize)]
pub(crate) struct Spec {
    pub exe: PathBuf,
    pub args: Vec<String>,
    pub input: String,
}

/// La única entrada de este modo es una tubería privada del padre. Sin shell.
pub fn worker_mode() {
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new("--job-worker")) {
        return;
    }
    let run = || -> Option<i32> {
        let s: Spec = serde_json::from_reader(std::io::stdin().take(1 << 20)).ok()?;
        let mut child = Command::new(s.exe)
            .args(s.args)
            .stdin(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()
            .ok()?;
        child.stdin.take()?.write_all(s.input.as_bytes()).ok()?;
        child.wait().ok()?.code()
    };
    std::process::exit(run().unwrap_or(1));
}

pub(crate) struct Job {
    handle: OwnedHandle,
    child: Child,
    pub bytes: Arc<AtomicU64>,
    tail: Arc<Mutex<Vec<u8>>>,
    readers: Vec<std::thread::JoinHandle<()>>,
}
fn ok(code: i32) -> Result<()> {
    if code == 0 {
        Err("Error de Job Win32".into())
    } else {
        Ok(())
    }
}
impl Job {
    pub fn launch(spec: Spec, cwd: &Path, env: &Env) -> Result<Self> {
        Self::spawn(spec, cwd, env, false)
    }
    pub fn captured(spec: Spec, cwd: &Path, env: &Env) -> Result<Self> {
        Self::spawn(spec, cwd, env, true)
    }
    fn spawn(spec: Spec, cwd: &Path, env: &Env, capture: bool) -> Result<Self> {
        unsafe {
            let raw = CreateJobObjectW(null(), null());
            if raw.is_null() {
                return Err("No se pudo crear Job".into());
            }
            let handle = OwnedHandle::from_raw_handle(raw);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            ok(SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ))?;
            let mut child = Command::new(std::env::current_exe().safe()?)
                .arg("--job-worker")
                .current_dir(cwd)
                .env_clear()
                .envs(env)
                .creation_flags(0x08000000)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .safe()?;
            if AssignProcessToJobObject(raw, child.as_raw_handle()) == 0 {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Asignación al Job denegada".into());
            }
            let bytes = Arc::new(AtomicU64::new(0));
            let tail = Arc::new(Mutex::new(Vec::new()));
            let mut readers = Vec::new();
            fn drain(
                mut src: impl Read + Send + 'static,
                count: Arc<AtomicU64>,
                tail: Option<Arc<Mutex<Vec<u8>>>>,
            ) -> std::thread::JoinHandle<()> {
                std::thread::spawn(move || {
                    let mut buf = [0; 8192];
                    while let Ok(n) = src.read(&mut buf) {
                        if n == 0 {
                            break;
                        }
                        count.fetch_add(n as u64, Ordering::Relaxed);
                        if let Some(tail) = &tail {
                            let mut tail = tail.lock().unwrap();
                            let overflow = (tail.len() + n).saturating_sub(16384);
                            tail.drain(..overflow);
                            tail.extend_from_slice(&buf[..n]);
                        }
                    }
                })
            }
            readers.push(drain(
                child.stdout.take().unwrap(),
                bytes.clone(),
                capture.then(|| tail.clone()),
            ));
            readers.push(drain(
                child.stderr.take().unwrap(),
                bytes.clone(),
                capture.then(|| tail.clone()),
            ));
            serde_json::to_writer(child.stdin.take().unwrap(), &spec).safe()?;
            Ok(Self {
                handle,
                child,
                bytes,
                tail,
                readers,
            })
        }
    }
    pub fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
    pub fn wait(&mut self, timeout: Duration) -> Result<(bool, String)> {
        let until = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().safe()? {
                if !self.pids()?.is_empty() {
                    self.stop()?;
                    return Err("Quedaron descendientes después de la preparación".into());
                }
                for thread in self.readers.drain(..) {
                    thread.join().safe()?;
                }
                return Ok((
                    status.success(),
                    String::from_utf8_lossy(&self.tail.lock().unwrap()).into_owned(),
                ));
            }
            if Instant::now() >= until {
                self.stop()?;
                return Err("Se agotó el plazo del proceso; su Job quedó vacío".into());
            }
            sleep(Duration::from_millis(100));
        }
    }
    pub fn pids(&self) -> Result<Vec<u32>> {
        unsafe {
            // Dos DWORD seguidos de ULONG_PTR[] en Windows x64.
            let mut buf = [0usize; 513];
            ok(QueryInformationJobObject(
                self.handle.as_raw_handle(),
                JobObjectBasicProcessIdList,
                buf.as_mut_ptr().cast(),
                size_of::<[usize; 513]>() as u32,
                null_mut(),
            ))?;
            let n = buf[0] >> 32;
            if n > 512 {
                return Err("Demasiados descendientes".into());
            }
            Ok(buf[1..1 + n].iter().map(|p| *p as u32).collect())
        }
    }
    pub fn metrics(&self) -> Option<(u64, f64)> {
        unsafe {
            let mut ext: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            let mut cpu: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = zeroed();
            ok(QueryInformationJobObject(
                self.handle.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&mut ext as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of_val(&ext) as u32,
                null_mut(),
            ))
            .ok()?;
            ok(QueryInformationJobObject(
                self.handle.as_raw_handle(),
                JobObjectBasicAccountingInformation,
                (&mut cpu as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                size_of_val(&cpu) as u32,
                null_mut(),
            ))
            .ok()?;
            Some((
                ext.PeakJobMemoryUsed as u64,
                (cpu.TotalUserTime + cpu.TotalKernelTime) as f64 / 1e7,
            ))
        }
    }
    pub fn stop(&mut self) -> Result<()> {
        unsafe { ok(TerminateJobObject(self.handle.as_raw_handle(), 0))? };
        for _ in 0..100 {
            if self.pids()?.is_empty() {
                return Ok(());
            }
            sleep(Duration::from_millis(50))
        }
        Err("Cierre del Job no confirmado".into())
    }
    /// Verifica listeners v4/v6 y, antes de credenciales, la conexión establecida exacta.
    pub fn owns(&self, port: u16, peer: Option<u16>) -> Result<bool> {
        let pids = self.pids()?;
        let mut listener = false;
        let mut connected = false;
        for (family, width, addr, local, remote, state, pid) in
            [(2, 24, 4, 8, 16, 0, 20), (23, 56, 0, 20, 44, 48, 52)]
        {
            let mut len = 0;
            unsafe { GetExtendedTcpTable(null_mut(), &mut len, 0, family, 5, 0) };
            if len < 4 {
                return Ok(false);
            }
            let mut buf = vec![0u32; (len as usize + 3) / 4];
            let code =
                unsafe { GetExtendedTcpTable(buf.as_mut_ptr().cast(), &mut len, 0, family, 5, 0) };
            if code == 122 {
                return Ok(false);
            }
            if code != 0 {
                return Err("Tabla TCP no disponible".into());
            }
            let bytes =
                unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), len as usize) };
            for row in bytes[4..].chunks_exact(width).take(buf[0] as usize) {
                let dword = |i: usize| u32::from_ne_bytes(row[i..i + 4].try_into().unwrap());
                let word = |i: usize| u16::from_be_bytes([row[i], row[i + 1]]);
                let mine = pids.contains(&dword(pid));
                let v4 = family == 2;
                let loopback = if v4 {
                    row[addr..addr + 4] == [127, 0, 0, 1]
                } else {
                    row[0..16] == std::net::Ipv6Addr::LOCALHOST.octets()
                };
                if dword(state) == 2 {
                    if mine && !loopback {
                        return Err("Listener del Job fuera de loopback".into());
                    }
                    if word(local) == port && v4 {
                        if !mine {
                            return Err("Puerto ocupado por otro proceso".into());
                        }
                        listener = loopback;
                    }
                } else if v4
                    && dword(state) == 5
                    && loopback
                    && word(local) == port
                    && Some(word(remote)) == peer
                    && row[12..16] == [127, 0, 0, 1]
                {
                    connected = mine;
                }
            }
        }
        Ok(listener && (peer.is_none() || connected))
    }
}

#[cfg(test)]
pub(crate) fn test_foreign(raw: windows_sys::Win32::Foundation::HANDLE, port: u16) {
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let child = Command::new("cmd.exe")
        .args(["/d", "/c", "exit", "0"])
        .spawn()
        .unwrap();
    let mut j = Job {
        handle,
        child,
        bytes: Arc::new(AtomicU64::new(0)),
        tail: Arc::default(),
        readers: vec![],
    };
    assert_eq!(
        j.owns(port, None).unwrap_err(),
        "Puerto ocupado por otro proceso"
    );
    j.child.wait().unwrap();
}
