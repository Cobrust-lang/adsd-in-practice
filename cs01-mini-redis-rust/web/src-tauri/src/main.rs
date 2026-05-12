#![forbid(unsafe_code)]

use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

const RESP_PORT: u16 = 6380;
const HTTP_PORT: u16 = 6381;

struct SidecarState {
    child: Mutex<Option<Child>>,
    status: Mutex<DesktopBackendStatus>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DesktopBackendKind {
    Starting,
    Running,
    Failed,
    Stopped,
}

#[derive(Clone, Debug, Serialize)]
struct DesktopBackendStatus {
    kind: DesktopBackendKind,
    resp_port: u16,
    http_port: u16,
    message: String,
}

impl DesktopBackendStatus {
    fn starting() -> Self {
        Self {
            kind: DesktopBackendKind::Starting,
            resp_port: RESP_PORT,
            http_port: HTTP_PORT,
            message: "Starting local redis-server sidecar on 127.0.0.1".to_string(),
        }
    }

    fn running(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopBackendKind::Running,
            resp_port: RESP_PORT,
            http_port: HTTP_PORT,
            message: message.into(),
        }
    }

    fn failed(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopBackendKind::Failed,
            resp_port: RESP_PORT,
            http_port: HTTP_PORT,
            message: message.into(),
        }
    }

    fn stopped() -> Self {
        Self {
            kind: DesktopBackendKind::Stopped,
            resp_port: RESP_PORT,
            http_port: HTTP_PORT,
            message: "Desktop shell stopped the local sidecar".to_string(),
        }
    }
}

#[tauri::command]
fn desktop_backend_status(state: tauri::State<'_, SidecarState>) -> DesktopBackendStatus {
    state
        .status
        .lock()
        .expect("desktop backend status mutex poisoned")
        .clone()
}

fn main() {
    tauri::Builder::default()
        .manage(SidecarState {
            child: Mutex::new(None),
            status: Mutex::new(DesktopBackendStatus::starting()),
        })
        .invoke_handler(tauri::generate_handler![desktop_backend_status])
        .setup(|app| {
            start_sidecar(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                stop_sidecar(window.app_handle());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running cs01 mini-redis desktop app");
}

fn start_sidecar(app: AppHandle) {
    let status = match sidecar_binary(&app) {
        Ok(binary) => spawn_sidecar(&app, binary),
        Err(error) => DesktopBackendStatus::failed(error),
    };
    set_status(&app, status);
}

fn sidecar_binary(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CS01_REDIS_SERVER_BIN") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "CS01_REDIS_SERVER_BIN points to a missing file: {}",
            path.display()
        ));
    }

    if let Ok(resource) = app.path().resolve("bin/redis-server", tauri::path::BaseDirectory::Resource)
    {
        if resource.is_file() {
            return Ok(resource);
        }
    }

    let candidates = dev_sidecar_candidates();
    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }

    let searched = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "redis-server sidecar not found. Build it with `cargo build -p redis-server`, or set CS01_REDIS_SERVER_BIN. Searched: {searched}"
    ))
}

fn dev_sidecar_candidates() -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir
        .parent()
        .and_then(|web| web.parent())
        .map(PathBuf::from);

    let mut candidates = Vec::new();
    if let Some(case_dir) = case_dir {
        candidates.push(case_dir.join("target/debug/redis-server"));
        candidates.push(case_dir.join("target/release/redis-server"));
    }
    candidates
}

fn spawn_sidecar(app: &AppHandle, binary: PathBuf) -> DesktopBackendStatus {
    if port_is_open(HTTP_PORT) {
        return DesktopBackendStatus::running(format!(
            "Reusing existing local HTTP control plane at http://127.0.0.1:{HTTP_PORT}"
        ));
    }

    let mut command = Command::new(&binary);
    command.args([
        "--bind",
        "127.0.0.1",
        "--port",
        &RESP_PORT.to_string(),
        "--http-port",
        &HTTP_PORT.to_string(),
    ]);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return DesktopBackendStatus::failed(format!(
                "failed to start redis-server sidecar {}: {error}",
                binary.display()
            ));
        }
    };

    {
        let state = app.state::<SidecarState>();
        *state
            .child
            .lock()
            .expect("desktop sidecar child mutex poisoned") = Some(child);
    }

    for _attempt in 0..40 {
        if port_is_open(HTTP_PORT) {
            return DesktopBackendStatus::running(format!(
                "Started redis-server sidecar from {} on 127.0.0.1:{RESP_PORT} (HTTP/SSE :{HTTP_PORT})",
                binary.display()
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    stop_sidecar(app);
    DesktopBackendStatus::failed(format!(
        "redis-server sidecar started but HTTP control plane did not become ready on 127.0.0.1:{HTTP_PORT} within 2s"
    ))
}

fn port_is_open(port: u16) -> bool {
    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    TcpStream::connect_timeout(&addr.into(), Duration::from_millis(100)).is_ok()
}

fn set_status(app: &AppHandle, status: DesktopBackendStatus) {
    let state = app.state::<SidecarState>();
    *state
        .status
        .lock()
        .expect("desktop backend status mutex poisoned") = status;
}

fn stop_sidecar(app: &AppHandle) {
    let state = app.state::<SidecarState>();
    let mut guard = state
        .child
        .lock()
        .expect("desktop sidecar child mutex poisoned");
    if let Some(mut child) = guard.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    drop(guard);
    set_status(app, DesktopBackendStatus::stopped());
}
