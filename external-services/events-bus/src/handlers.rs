// Event handlers stub
pub trait EventHandler {
    fn handle_event(&self, event: &crate::event::Event) -> crate::error::Result<()>;
}