// Configuration providers stub (file, env, etcd, etc.)
pub trait ConfigProvider {
    fn load(&self) -> crate::error::Result<serde_json::Value>;
}