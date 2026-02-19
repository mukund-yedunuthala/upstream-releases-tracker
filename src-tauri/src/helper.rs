use crate::config_handler::ConfigHandler;
use tracker_libs::Config;

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn read_config() -> Result<Config, String> {
    ConfigHandler::read_config()
        .map_err(|e| format!("Failed to read config: {}", e))
}
