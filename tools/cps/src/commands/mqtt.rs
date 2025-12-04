use crate::blockchain::Config;
use crate::mqtt;
use anyhow::Result;

pub async fn subscribe(
    _blockchain_config: &Config,
    _mqtt_config: &mqtt::Config,
    _topic: &str,
    _node_id: u64,
    _encrypt: bool,
) -> Result<()> {
    crate::display::tree::error("MQTT subscribe not yet implemented");
    Ok(())
}

pub async fn publish(
    _blockchain_config: &Config,
    _mqtt_config: &mqtt::Config,
    _topic: &str,
    _node_id: u64,
    _interval: u64,
) -> Result<()> {
    crate::display::tree::error("MQTT publish not yet implemented");
    Ok(())
}
