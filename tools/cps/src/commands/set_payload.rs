use crate::blockchain::Config;
use anyhow::Result;

pub async fn execute(_config: &Config, _node_id: u64, _data: String, _encrypt: bool) -> Result<()> {
    crate::display::tree::error("Command not yet implemented");
    Ok(())
}
