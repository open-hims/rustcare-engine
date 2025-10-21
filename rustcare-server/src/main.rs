use axum::{
    middleware::from_fn,
    Router,
};
use clap::Parser;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth;
mod handlers;
mod middleware;
mod routes;
mod server;
mod openapi;
mod security_state;
// mod grpc; // Disabled temporarily

use middleware as app_middleware;
use security_state::SecurityState;

use crate::server::RustCareServer;
use error_common::{RustCareError, Result};
// use logger_redacted::RedactedLogger;

/// RustCare Engine HTTP Server
#[derive(Parser, Debug)]
#[command(name = "rustcare-server")]
#[command(about = "HIPAA-compliant healthcare platform HTTP API server")]
struct Args {
    /// Server bind address
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Server port
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Configuration file path
    #[arg(short, long, default_value = "rustcare-server.yaml")]
    config: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Enable HIPAA audit mode
    #[arg(long)]
    hipaa_audit: bool,

    /// Enable gRPC server
    #[arg(long)]
    enable_grpc: bool,

    /// gRPC server port
    #[arg(long, default_value = "9090")]
    grpc_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing with our redacted logger
    init_tracing(args.verbose).await?;

    info!("🏥 Starting RustCare Engine HTTP Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Bind address: {}:{}", args.host, args.port);

    // Initialize security configuration
    info!("🔐 Initializing security subsystems...");
    match SecurityState::from_env().await {
        Ok(security) => {
            security.print_summary();
            info!("✅ Security initialization complete");
            // TODO: Store security state in server context
        }
        Err(e) => {
            tracing::error!("❌ Security initialization failed: {}", e);
            tracing::error!("   Please check your .env file and environment variables");
            return Err(RustCareError::InternalError(format!("Security init failed: {}", e)));
        }
    }

    // Initialize redacted logger for HIPAA compliance
    // TODO: Initialize RedactedLogger properly
    // let _redacted_logger = RedactedLogger::new("rustcare_server").await;

    // Initialize the RustCare server
    let server = RustCareServer::new(&args.config).await?;
    
    // Create the router with all routes
    let app = create_app(server).await?;

    // Start gRPC server if enabled (disabled temporarily)
    let grpc_handle: Option<tokio::task::JoinHandle<()>> = None;
    
    if args.enable_grpc {
        tracing::warn!("gRPC server is temporarily disabled for testing");
    }

    // Bind and serve HTTP server
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| RustCareError::NetworkError(format!("Failed to bind to {}: {}", addr, e)))?;
    
    info!("🚀 RustCare Engine server running on http://{}:{}", args.host, args.port);
    info!("📋 Health check available at: http://{}:{}/health", args.host, args.port);
    info!("📋 API v1 available at: http://{}:{}/api/v1", args.host, args.port);
    info!("🔐 Authentication endpoints: http://{}:{}/api/v1/auth", args.host, args.port);
    info!("⚙️  Workflow endpoints: http://{}:{}/api/v1/workflow", args.host, args.port);
    info!("🔌 WebSocket endpoints: ws://{}:{}/ws", args.host, args.port);
    
    if args.enable_grpc {
        info!("🔧 gRPC server available on grpc://{}:{}", args.host, args.grpc_port);
    }

    // Run HTTP server
    let http_result = axum::serve(listener, app).await
        .map_err(|e| RustCareError::ServerError(format!("HTTP server error: {}", e)));

    // Wait for gRPC server to finish if it was started
    if let Some(handle) = grpc_handle {
        let _ = handle.await;
    }

    http_result?;
    Ok(())
}

async fn create_app(server: RustCareServer) -> Result<Router> {
    let app = routes::create_routes()
        // Add middleware layers
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(app_middleware::create_cors_layer())
                .layer(from_fn(app_middleware::request_timing_middleware))
                .layer(from_fn(app_middleware::audit_logging_middleware))
        )
        .with_state(server);

    Ok(app)
}



async fn init_tracing(verbose: bool) -> Result<()> {
    let level = if verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // Initialize with HIPAA-compliant logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("rustcare_server={},tower_http=info", level).into())
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .json() // Use JSON format for structured logging
        )
        .init();

    Ok(())
}