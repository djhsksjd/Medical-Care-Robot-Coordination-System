//! HTTP API entry point for the scheduler demo.
//! 这里相当于「用户态入口」，启动一个基于 axum 的 HTTP 服务器，
//! 把内核协调器的状态以 JSON 形式暴露给前端 Dashboard 使用。

use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use COMP_2432project::api::{AppState, build_router};

#[tokio::main]
async fn main() -> ExitCode {
    let state = AppState::new();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app: Router = build_router(state).layer(cors);

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3000);
    let addr: SocketAddr = format!("0.0.0.0:{port}")
        .parse()
        .expect("valid socket address");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "Failed to start HTTP API: port {port} is already in use. Stop the existing server or run with PORT=<other-port>."
            );
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("Failed to start HTTP API on http://{addr}: {error}");
            return ExitCode::FAILURE;
        }
    };

    println!("HTTP API server listening on http://{addr}");

    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("HTTP API server exited with error: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
