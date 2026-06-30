use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use base64::Engine;
use rand::Rng;
use std::{fs, net::SocketAddr, path::PathBuf, process::Command};
use tokio::net::TcpListener;

const MAX_BODY: usize = 10 * 1024 * 1024;
const ID_LEN: usize = 8;
const ID_ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const DEFAULT_PASSWORD: &str = "changeme";

#[derive(Clone)]
struct AppState {
    data_dir: PathBuf,
    // Fixed advertised base URL (from BASE_URL). When None, derive it per-request
    // from the Host / X-Forwarded-* headers so it matches how the client reached us.
    base_url: Option<String>,
    password: String,
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7777);
    let data_dir =
        PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "./screenshots".to_string()));
    fs::create_dir_all(&data_dir).expect("create data dir");

    let base_url = std::env::var("BASE_URL").ok().filter(|s| !s.is_empty());
    let password = std::env::var("PASSWORD").unwrap_or_else(|_| DEFAULT_PASSWORD.to_string());

    // `make url` greps for this line; print the fixed URL or a tailnet hint.
    println!(
        "Server URL: {}",
        base_url.clone().unwrap_or_else(|| detect_base_url(port))
    );
    if base_url.is_none() {
        println!("(BASE_URL unset — advertised URL is derived from each request)");
    }
    println!("Data dir:   {}", data_dir.display());
    if password == DEFAULT_PASSWORD {
        eprintln!(
            "warning: using the default password '{}'. Set PASSWORD to override.",
            DEFAULT_PASSWORD
        );
    }

    let state = AppState {
        data_dir,
        base_url,
        password,
    };

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        // OPTIONS handles the CORS preflight a browser sends before a POST with
        // a non-simple Content-Type (e.g. image/png).
        .route("/upload", post(upload).options(preflight))
        .route("/s/:filename", get(serve))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        // Attach permissive CORS headers to every response. The tailnet is the
        // perimeter and there's no auth, so allowing any origin costs nothing
        // and lets browser extensions (Firefox MV3 especially) upload.
        .layer(middleware::map_response(add_cors_headers))
        .with_state(state);

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = TcpListener::bind(addr).await.expect("bind");
    println!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}

async fn preflight() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

// HTTP Basic auth. The username is ignored; only the password after the colon
// is checked. Returns true when the request carries the right password.
fn authorized(headers: &HeaderMap, password: &str) -> bool {
    let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(b64) = auth.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) else {
        return false;
    };
    let Ok(creds) = String::from_utf8(decoded) else {
        return false;
    };
    let supplied = creds
        .split_once(':')
        .map(|(_, password)| password)
        .unwrap_or("");
    constant_time_eq(supplied.as_bytes(), password.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// Where to advertise served files. Prefer the fixed BASE_URL; otherwise rebuild
// it from how this request arrived (scheme + host), so links work whether the
// client reached us over the tailnet or a public HTTPS proxy.
fn request_base_url(state: &AppState, headers: &HeaderMap) -> String {
    if let Some(base) = &state.base_url {
        return base.clone();
    }
    let first = |v: &str| v.split(',').next().unwrap_or("").trim().to_string();
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(first)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http".to_string());
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|v| v.to_str().ok())
        .map(first)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "localhost".to_string());
    format!("{}://{}", proto, host)
}

async fn add_cors_headers(mut res: Response) -> Response {
    let h = res.headers_mut();
    h.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("*"),
    );
    h.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("86400"),
    );
    res
}

fn detect_base_url(port: u16) -> String {
    if let Ok(out) = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
    {
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
    if !authorized(&headers, &state.password) {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                "Basic realm=\"tailscale-screenshot\"",
            )],
            "unauthorized\n",
        )
            .into_response();
    }
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
    let url = format!("{}/s/{}", request_base_url(&state, &headers), filename);
    let body = serde_json::json!({ "url": url, "id": id }).to_string();
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

async fn serve(State(state): State<AppState>, Path(filename): Path<String>) -> Response {
    let Some((id, ext)) = filename.rsplit_once('.') else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
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
