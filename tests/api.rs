use std::{net::SocketAddr, sync::Arc};

use axum::{Router, routing::get};
use check_host_worker::{
    app::{self, AppState},
    config::{Api, Config, Location, Server},
};
use serde_json::{Value, json};
use tokio::net::{TcpListener, UdpSocket};

const API_KEY: &str = "test-key";

async fn serve(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    addr
}

async fn spawn_app() -> SocketAddr {
    let config = Config {
        server: Server { port: 0 },
        api: Api {
            key: API_KEY.to_owned(),
        },
        location: Location {
            city: "Paris".to_owned(),
            country: "France".to_owned(),
        },
    };
    serve(app::router(Arc::new(AppState::new(config).unwrap()))).await
}

async fn get_json(url: String) -> (u16, Value) {
    let response = reqwest::get(url).await.unwrap();
    let status = response.status().as_u16();
    let body = response.text().await.unwrap();
    (status, serde_json::from_str(&body).unwrap())
}

async fn unused_tcp_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

fn assert_probe_succeeded(body: &Value, address: &str) {
    assert_eq!(body["success"], true, "{body}");
    assert_eq!(body["address"], address);
    assert!(body["elapsed_ms"].is_u64());
    assert!(body.get("error").is_none());
}

fn assert_probe_failed(body: &Value, address: &str) {
    assert_eq!(body["success"], false, "{body}");
    assert_eq!(body["address"], address);
    assert!(body["error"].is_string());
    assert!(body.get("elapsed_ms").is_none());
}

#[tokio::test]
async fn rejects_missing_api_key() {
    let app = spawn_app().await;
    let (status, body) = get_json(format!("http://{app}/tcp/127.0.0.1:1")).await;
    assert_eq!(status, 401);
    assert_eq!(body, json!({ "error": "Missing API key" }));
}

#[tokio::test]
async fn rejects_invalid_api_key() {
    let app = spawn_app().await;
    let (status, body) = get_json(format!("http://{app}/tcp/127.0.0.1:1?key=wrong")).await;
    assert_eq!(status, 401);
    assert_eq!(body, json!({ "error": "Invalid API key" }));
}

#[tokio::test]
async fn tcp_probe_reports_open_and_closed_ports() {
    let app = spawn_app().await;
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let open = target.local_addr().unwrap().to_string();
    let (status, body) = get_json(format!("http://{app}/tcp/{open}?key={API_KEY}")).await;
    assert_eq!(status, 200);
    assert_probe_succeeded(&body, &open);

    let closed = format!("127.0.0.1:{}", unused_tcp_port().await);
    let (status, body) = get_json(format!("http://{app}/tcp/{closed}?key={API_KEY}")).await;
    assert_eq!(status, 200);
    assert_probe_failed(&body, &closed);
}

#[tokio::test]
async fn udp_probe_measures_echoed_datagram() {
    let app = spawn_app().await;
    let target = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let address = target.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let (size, source) = target.recv_from(&mut buffer).await.unwrap();
        target.send_to(&buffer[..size], source).await.unwrap();
    });

    let (status, body) = get_json(format!("http://{app}/udp/{address}?key={API_KEY}")).await;
    assert_eq!(status, 200);
    assert_probe_succeeded(&body, &address);
}

#[tokio::test]
async fn http_probe_reports_reachable_and_unreachable_urls() {
    let app = spawn_app().await;
    let target = serve(Router::new().route("/", get(|| async { "ok" }))).await;
    let reachable = format!("http://{target}/");
    let (status, body) =
        get_json(format!("http://{app}/http?key={API_KEY}&addr={reachable}")).await;
    assert_eq!(status, 200);
    assert_probe_succeeded(&body, &reachable);

    let unreachable = format!("http://127.0.0.1:{}/", unused_tcp_port().await);
    let (status, body) = get_json(format!(
        "http://{app}/http?key={API_KEY}&addr={unreachable}"
    ))
    .await;
    assert_eq!(status, 200);
    assert_probe_failed(&body, &unreachable);
}

#[tokio::test]
async fn ping_reports_invalid_ip_as_failure_with_location() {
    let app = spawn_app().await;
    let (status, body) = get_json(format!("http://{app}/ping/not-an-ip?key={API_KEY}")).await;
    assert_eq!(status, 200);
    assert_eq!(body["ip"], "not-an-ip");
    assert_eq!(body["success"], false);
    assert_eq!(body["rtt_ms"], 0);
    assert_eq!(body["location"], "Paris, France");
    assert!(body["message"].is_string());
}
