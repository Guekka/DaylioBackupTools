use axum::{Router, routing::get};

pub async fn serve(host: String, port: u16) -> color_eyre::Result<()> {
    println!("Starting server on port {port}...");

    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind(host + ":" + &port.to_string()).await?;

    axum::serve(listener, app).await?;

    Ok(())
}
