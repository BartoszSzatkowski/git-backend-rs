use axum::middleware::Next;
use axum::{
    body::{to_bytes, Body, Bytes},
    extract::{Query, Request},
    http::{header, header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use git_backend_rs::PktLine;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{BufRead, Read};
use std::process::{Command, Stdio};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/info/refs", get(refs))
        .route("/git-upload-pack", post(upload_pack))
        // .route("/git-upload-archive", post(refs))
        // .route("/git-receive-pack", post(refs))
        .layer(middleware::from_fn(print_request));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Deserialize)]
struct Params {
    service: String,
}

async fn refs(Query(Params { service }): Query<Params>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();

    let a = Command::new("/usr/lib/git-core/git-upload-pack")
        .arg("--advertise-refs")
        .arg("/home/bartosz/dotfiles")
        .output()
        .expect("failed to execute process");

    let service_header = format!("application/x-{}-advertisement", service);
    headers.insert(header::CONTENT_TYPE, service_header.parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());

    let service_desc = format!("# service={service}\n");
    let service_ptk_line = PktLine(&service_desc);
    let res = format!(
        "{}0000{}",
        service_ptk_line,
        String::from_utf8_lossy(&a.stdout)
    );

    (headers, res)
}

async fn upload_pack(req_headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    tracing::debug!("inside upload_pack");

    let mut cmd = Command::new("git");
    cmd.arg("http-backend");
    // Required environment variables
    cmd.env("REQUEST_METHOD", "POST");
    cmd.env("GIT_PROJECT_ROOT", "/home/bartosz/dotfiles");
    cmd.env("PATH_INFO", "/git-upload-pack");

    cmd.env("REMOTE_USER", "");
    cmd.env("REMOTE_ADDR", "0.0.0.0");
    cmd.env("QUERY_STRING", "");
    cmd.env(
        "CONTENT_TYPE",
        req_headers.get(CONTENT_TYPE).unwrap().to_str().unwrap(),
    );
    cmd.stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped());
    let mut p = cmd.spawn().unwrap();
    let _ = std::io::copy(&mut body.to_vec().as_slice(), &mut p.stdin.take().unwrap());
    let out = p.wait_with_output().unwrap();
    let mut rdr = std::io::BufReader::new(std::io::Cursor::new(out.stdout));

    let mut headers = HashMap::new();
    for line in rdr.by_ref().lines() {
        let line = match line {
            Ok(s) => s,
            _ => break,
        };
        if line.is_empty() || line == "\r" {
            break;
        }

        let (key, value) = line.split_once(':').unwrap();
        // let key = parts.next().unwrap();
        // let value = parts.next().unwrap();
        let value = &value[1..];
        headers.insert(key.to_string(), value.to_string());
    }
    let mut b = Vec::new();
    rdr.read_to_end(&mut b).unwrap();
    let h: HeaderMap = (&headers).try_into().expect("valid headers");

    (h, Body::from(b))
}

async fn print_request(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::debug!("{:#?}", &req);
    let (parts, body) = req.into_parts();
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    Ok(res)
}

async fn buffer_and_print(direction: &str, body: Body) -> Result<Bytes, (StatusCode, String)> {
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {} body: {}", direction, err),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{} body = {:?}", direction, body);
    }

    Ok(bytes)
}
