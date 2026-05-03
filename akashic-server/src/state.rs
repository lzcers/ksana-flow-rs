#[derive(Debug, Clone)]
pub struct AppState {
    pub service_name: &'static str,
    pub api_version: &'static str,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            service_name: "akashic-server",
            api_version: "v0-draft",
        }
    }
}
