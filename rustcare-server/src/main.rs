use axum::{
    middleware::from_fn,
    Router,
};
use clap::Parser;
use colored::*;
use std::{io, net::SocketAddr, env};
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    field::RecordFields,
    EnvFilter,
};
use tracing_subscriber::fmt::FormatFields;

use rustcare_server::{create_app, RustCareServer, SecurityState};
use error_common::{RustCareError, Result};

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

    info!("🏥 {}", "Starting RustCare Engine HTTP Server".bright_cyan());
    info!("📋 Version: {}", env!("CARGO_PKG_VERSION").bright_white());
    info!("🌐 Bind address: {}", format!("{}:{}", args.host, args.port).bright_yellow());

    // Initialize security configuration
    info!("🔐 {}", "Initializing security subsystems...".bright_cyan());
    match SecurityState::from_env().await {
        Ok(security) => {
            security.print_summary();
            info!("✅ {}", "Security initialization complete".bright_green());
            // TODO: Store security state in server context
        }
        Err(e) => {
            tracing::error!("❌ {}: {}", "Security initialization failed".bright_red(), e);
            tracing::error!("   {}", "Please check your .env file and environment variables".bright_yellow());
            return Err(RustCareError::InternalError(format!("Security init failed: {}", e)));
        }
    }

    // Initialize redacted logger for HIPAA compliance
    // TODO: Initialize RedactedLogger properly
    // let _redacted_logger = RedactedLogger::new("rustcare_server").await;

    // Initialize the RustCare server
    let server = RustCareServer::new(&args.config).await?;
    
    // Create the router with all routes
    let app = create_app(server);

    // Start gRPC server if enabled (disabled temporarily)
    let grpc_handle: Option<tokio::task::JoinHandle<()>> = None;
    
    if args.enable_grpc {
        tracing::warn!("gRPC server is temporarily disabled for testing");
    }

    // Bind and serve HTTP server
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| RustCareError::NetworkError(format!("Failed to bind to {}: {}", addr, e)))?;
    
    info!("🚀 {}", format!("RustCare Engine server running on http://{}:{}", args.host, args.port).bright_green());
    info!("📋 {}", format!("Health check available at: http://{}:{}/health", args.host, args.port).bright_blue());
    info!("📋 {}", format!("API v1 available at: http://{}:{}/api/v1", args.host, args.port).bright_blue());
    info!("🔐 {}", format!("Authentication endpoints: http://{}:{}/api/v1/auth", args.host, args.port).bright_blue());
    info!("⚙️  {}", format!("Workflow endpoints: http://{}:{}/api/v1/workflow", args.host, args.port).bright_blue());
    info!("🔌 {}", format!("WebSocket endpoints: ws://{}:{}/ws", args.host, args.port).bright_blue());
    
    if args.enable_grpc {
        info!("🔧 {}", format!("gRPC server available on grpc://{}:{}", args.host, args.grpc_port).bright_purple());
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

async fn init_tracing(verbose: bool) -> Result<()> {
    let level = if verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // Check if we're in development or production
    let is_development = env::var("RUSTCARE_ENV").unwrap_or_else(|_| "development".to_string()) == "development";
    let use_colors = env::var("NO_COLOR").is_err() && atty::is(atty::Stream::Stdout);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            format!(
                "rustcare_server={},tower_http=info,sqlx=warn,hyper=info,reqwest=info",
                level
            ).into()
        });

    if is_development && use_colors {
        // Beautiful colored development logging
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_timer(ChronoUtc::rfc_3339())
                    .with_ansi(true)
                    .with_level(true)
                    .event_format(ColoredFormatter::new())
                    .fmt_fields(ColoredFieldFormatter::new())
            )
            .init();

        // Print a beautiful startup banner
        print_startup_banner();
    } else {
        // Structured JSON logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_timer(ChronoUtc::rfc_3339())
                    .with_ansi(false)
                    .json()
            )
            .init();
    }

    Ok(())
}

fn print_startup_banner() {
    println!("{}", "╔══════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║                        🏥 RUSTCARE ENGINE                    ║".bright_cyan());
    println!("{}", "║                  HIPAA-Compliant Healthcare Platform         ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
}

// Custom colored formatter for development
struct ColoredFormatter {
    timer: ChronoUtc,
}

impl ColoredFormatter {
    fn new() -> Self {
        Self {
            timer: ChronoUtc::rfc_3339(),
        }
    }
}

impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for ColoredFormatter
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.metadata();
        
        // Format timestamp
        write!(writer, "{} ", chrono::Utc::now().format("%H:%M:%S%.3f").to_string().bright_black())?;
        
        // Format level with colors
        let level_str = match *metadata.level() {
            Level::TRACE => "TRACE".bright_purple(),
            Level::DEBUG => "DEBUG".bright_blue(),
            Level::INFO => " INFO".bright_green(),
            Level::WARN => " WARN".bright_yellow(),
            Level::ERROR => "ERROR".bright_red(),
        };
        write!(writer, "[{}] ", level_str)?;

        // Format target/module
        if let Some(target) = metadata.target().split("::").last() {
            write!(writer, "{:<15} ", target.bright_cyan())?;
        }

        // Format thread info
        if let Some(thread_name) = std::thread::current().name() {
            if thread_name != "main" {
                write!(writer, "({}) ", thread_name.bright_magenta())?;
            }
        }

        // Format the actual message
        ctx.format_fields(writer.by_ref(), event)?;

        // Add file and line info for debug/trace
        if metadata.level() <= &Level::DEBUG {
            if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
                let file_short = file.split('/').last().unwrap_or(file);
                write!(writer, " {}{}:{}{}", 
                    "(".bright_black(), 
                    file_short.bright_black(), 
                    line.to_string().bright_black(),
                    ")".bright_black()
                )?;
            }
        }

        writeln!(writer)
    }
}

// Custom field formatter for colored output
struct ColoredFieldFormatter;

impl ColoredFieldFormatter {
    fn new() -> Self {
        Self
    }
}

impl<'a> tracing_subscriber::fmt::FormatFields<'a> for ColoredFieldFormatter {
    fn format_fields<R: RecordFields>(
        &self,
        writer: tracing_subscriber::fmt::format::Writer<'_>,
        fields: R,
    ) -> std::fmt::Result {
        let mut visitor = ColoredFieldVisitor {
            writer,
            is_first: true,
        };
        fields.record(&mut visitor);
        Ok(())
    }
}

struct ColoredFieldVisitor<'a> {
    writer: tracing_subscriber::fmt::format::Writer<'a>,
    is_first: bool,
}

impl<'a> tracing::field::Visit for ColoredFieldVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // For message field, try to extract the actual text if it's a ColoredString
            let debug_str = format!("{:?}", value);
            if debug_str.contains("ColoredString") {
                // Extract the actual text from ColoredString debug output
                if let Some(start) = debug_str.find("input: \"") {
                    if let Some(end) = debug_str[start + 8..].find("\"") {
                        let actual_text = &debug_str[start + 8..start + 8 + end];
                        write!(self.writer, "{}", actual_text.white().bold()).unwrap();
                        return;
                    }
                }
            }
            write!(self.writer, "{}", debug_str.white().bold()).unwrap();
        } else {
            if !self.is_first {
                write!(self.writer, " ").unwrap();
            }
            write!(
                self.writer,
                "{}{}={}", 
                if self.is_first { "" } else { " " },
                field.name().bright_yellow(),
                format!("{:?}", value).bright_white()
            ).unwrap();
        }
        self.is_first = false;
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            write!(self.writer, "{}", value.white().bold()).unwrap();
        } else {
            if !self.is_first {
                write!(self.writer, " ").unwrap();
            }
            write!(
                self.writer,
                "{}{}={}", 
                if self.is_first { "" } else { " " },
                field.name().bright_yellow(),
                value.bright_white()
            ).unwrap();
        }
        self.is_first = false;
    }
}