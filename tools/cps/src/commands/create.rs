use crate::blockchain::Config;
use anyhow::Result;

pub async fn execute(
    _config: &Config,
    _parent: Option<u64>,
    _meta: Option<String>,
    _payload: Option<String>,
    _encrypt: bool,
) -> Result<()> {
    crate::display::tree::error("Command not yet implemented");
    Ok(())
}
