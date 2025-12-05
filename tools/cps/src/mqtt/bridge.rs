
//! MQTT broker configuration.

/// Configuration for MQTT broker connection.
///
/// This configuration is used to establish connections to MQTT brokers
/// for IoT device integration.
///
/// # Examples
///
/// ```
/// use libcps::mqtt::Config;
///
/// // Anonymous connection
/// let config = Config {
///     broker: "mqtt://localhost:1883".to_string(),
///     username: None,
///     password: None,
///     client_id: None,
/// };
///
/// // Authenticated connection
/// let config_auth = Config {
///     broker: "mqtt://broker.example.com:1883".to_string(),
///     username: Some("myuser".to_string()),
///     password: Some("mypass".to_string()),
///     client_id: Some("cps-client".to_string()),
/// };
/// ```
#[derive(Clone)]
pub struct Config {
    /// MQTT broker URL (e.g., "mqtt://localhost:1883")
    pub broker: String,
    /// Optional username for authentication
    pub username: Option<String>,
    /// Optional password for authentication
    pub password: Option<String>,
    /// Optional client ID for MQTT connection
    pub client_id: Option<String>,
}
