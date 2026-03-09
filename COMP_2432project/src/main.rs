//! HTTP API entry point for the scheduler demo.
//! 这里相当于「用户态入口」，启动一个基于 axum 的 HTTP 服务器，
//! 把内核协调器的状态以 JSON 形式暴露给前端 Dashboard 使用。

use std::net::SocketAddr;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use COMP_2432project::api::{build_router, AppState};

#[tokio::main]
async fn main() {
    let state = AppState::new();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app: Router = build_router(state).layer(cors);

    let addr: SocketAddr = "0.0.0.0:3000".parse().expect("valid socket address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind server");
    println!("HTTP API server listening on http://{addr}");

    axum::serve(listener, app).await.expect("serve HTTP");
}
