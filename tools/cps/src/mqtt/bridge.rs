///////////////////////////////////////////////////////////////////////////////
//
//  Copyright 2018-2025 Robonomics Network <research@robonomics.network>
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//
///////////////////////////////////////////////////////////////////////////////
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
