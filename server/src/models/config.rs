use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// # App configuration
///
/// Contains dynamic app configuration used in the UI, such as the server/instance name.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Name of this IAM server instance, used as a title in the UI
    pub instance_name: String,
}
