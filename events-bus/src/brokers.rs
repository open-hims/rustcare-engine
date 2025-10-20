// Event brokers stub (Kafka, RabbitMQ, Redis, etc.)
pub trait EventBroker {
    async fn connect(&self) -> crate::error::Result<()>;
    async fn disconnect(&self) -> crate::error::Result<()>;
}