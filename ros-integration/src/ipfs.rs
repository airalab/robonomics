use std::sync::Mutex;
use ipfs_api::IpfsClient;
use std::io::Write;
use futures::{Stream};
use std::fs::File;
use substrate_service::{TaskExecutor};
use futures::{Future};

lazy_static! {
    static ref IPFS: Mutex<Option<IpfsClient>> = Mutex::new(None);
}

macro_rules! ipfs {
    () => {
        IPFS.lock().unwrap().as_mut().unwrap()
    };
}

#[inline]
pub fn init() {
    let client = Some(IpfsClient::default());
    let mut ipfs = IPFS.lock().unwrap();
    *ipfs = client;
}

#[inline]
pub fn read_file(ipfs_path: &str) -> impl Future<Item=(),Error=()> {
    let mut f = File::create(ipfs_path).expect("could not create file");

    ipfs!().cat(ipfs_path)
        .for_each(move |chunk| f.write_all(&chunk).map_err(From::from))
        .map_err(|e| eprintln!("{}", e))
}