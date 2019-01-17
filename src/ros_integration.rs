use rosrust::api::Ros;
use futures::sync::oneshot;
pub use substrate_cli::error;
use cli::Exit;

pub fn new(name: &str) -> error::Result<()> {
    let _ros = Ros::new(name).unwrap();
    Ok(())
}
