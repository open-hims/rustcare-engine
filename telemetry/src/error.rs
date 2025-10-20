use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Metrics collection failed")]
    MetricsError,
    
    #[error("Tracing initialization failed")]
    TracingError,
    
    #[error("Health check failed")]
    HealthCheckError,
    
    #[error("Alert system error")]
    AlertError,
    
    #[error("Dashboard error")]
    DashboardError,
    
    #[error("Exporter error")]
    ExporterError,
    
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, TelemetryError>;