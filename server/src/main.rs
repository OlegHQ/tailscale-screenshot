use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use rand::Rng;
use std::{fs, net::SocketAddr, path::PathBuf, process::Command};
use tokio::net::TcpListener;

const MAX_BODY: usize = 10 * 1024 * 1024;
const ID_LEN: usize = 8;
const ID_ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[derive(Clone)]
struct AppState {
    data_dir: PathBuf,
    base_url: String,
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7777);
    let data_dir = PathBuf::from(
        std::env::var("DATA_DIR").unwrap_or_else(|_| "./screenshots".to_string()),
    );
    fs::create_dir_all(&data_dir).expect("create data dir");

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| detect_base_url(port));

    println!("Server URL: {}", base_url);
    println!("Data dir:   {}", data_dir.display());

    let state = AppState { data_dir, base_url };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/upload", post(upload))
        .route("/s/:filename", get(serve))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state);

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = TcpListener::bind(addr).await.expect("bind");
    println!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

fn detect_base_url(port: u16) -> String {
    if let Ok(out) = Command::new("tailscale").args(["status", "--json"]).output() {
        if out.status.success() {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(name) = v
                    .get("Self")
                    .and_then(|s| s.get("DNSName"))
                    .and_then(|n| n.as_str())
                {
                    let host = name.trim_end_matches('.');
                    if !host.is_empty() {
                        return format!("http://{}:{}", host, port);
                    }
                }
            }
        }
    }
    if let Ok(out) = Command::new("tailscale").args(["ip", "-4"]).output() {
        if out.status.success() {
            if let Some(ip) = std::str::from_utf8(&out.stdout)
                .ok()
                .and_then(|s| s.lines().next())
                .map(|s| s.trim())
            {
                if !ip.is_empty() {
                    return format!("http://{}:{}", ip, port);
                }
            }
        }
    }
    eprintln!("warning: tailscale not reachable, using localhost");
    format!("http://127.0.0.1:{}", port)
}

fn ext_for_mime(mime: &str) -> Option<&'static str> {
    match mime.split(';').next().unwrap_or("").trim() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

fn mime_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn random_id() -> String {
    let mut rng = rand::thread_rng();
    (0..ID_LEN)
        .map(|_| ID_ALPHA[rng.gen_range(0..ID_ALPHA.len())] as char)
        .collect()
}

async fn upload(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let Some(ext) = ext_for_mime(ct) else {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            format!("unsupported content-type: {}\n", ct),
        )
            .into_response();
    };
    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty body\n").into_response();
    }
    let id = random_id();
    let filename = format!("{}.{}", id, ext);
    let path = state.data_dir.join(&filename);
    if let Err(e) = fs::write(&path, &body) {
        eprintln!("write failed: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "write failed\n").into_response();
    }
    let url = format!("{}/s/{}", state.base_url, filename);
    let body = serde_json::json!({ "url": url, "id": id }).to_string();
    (
        [(header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

async fn serve(State(state): State<AppState>, Path(filename): Path<String>) -> Response {
    let Some((id, ext)) = filename.rsplit_once('.') else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if id.is_empty()
        || id.len() > 32
        || !id.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(mime) = mime_for_ext(ext) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = state.data_dir.join(&filename);
    let Ok(bytes) = fs::read(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        bytes,
    )
        .into_response()
}
