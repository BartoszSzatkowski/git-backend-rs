use axum::middleware::Next;
use axum::{
    extract::{Query, Request},
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use git_backend_rs::PktLine;
use serde::Deserialize;
use std::process::Command;
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
        .route("/git-upload-pack", post(refs))
        .route("/git-upload-archive", post(refs))
        .route("/git-receive-pack", post(refs))
        .route("/*pp", post(|| async { "HELLO" }))
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

async fn print_request(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::debug!("{:#?}", &req);

    let res = next.run(req).await;

    Ok(res)
}
