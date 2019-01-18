use futures::{Future, Stream};
use network::SyncProvider;
use tokio::timer::Interval;
use tokio::runtime::TaskExecutor;
use std::time::{Duration, Instant};
use runtime_primitives::traits::As;
use substrate_service::{Service, Components};

use msg::std_msgs::UInt64;
use rosrust::api::Ros;

const TIMER_INTERVAL_MS: u64 = 5000;

pub fn start<C>(service: &Service<C>, exit: ::exit_future::Exit, handle: TaskExecutor)
    where C: Components
{
    let mut ros = Ros::new("robonomics").unwrap();
    let mut block_number_pub = ros.publish("block_number").unwrap();
    let mut num_peers_pub = ros.publish("num_peers").unwrap();

	let network = service.network();
    let client = service.client();

	let interval = Interval::new(Instant::now(), Duration::from_millis(TIMER_INTERVAL_MS));
    let status_publish = interval.map_err(|e| debug!("Timer error: {:?}", e)).for_each(move |_| {
		let sync_status = network.status();
		if let Ok(info) = client.info() {
            let mut block_number_msg = UInt64::default();
		    block_number_msg.data = info.chain.best_number.as_();
            block_number_pub.send(block_number_msg).unwrap();

            let mut num_peers_msg = UInt64::default();
            num_peers_msg.data = sync_status.num_peers as u64;
            num_peers_pub.send(num_peers_msg).unwrap();
		}
        Ok(())
    });

    handle.spawn(exit.until(status_publish).map(|_| ()));
}
