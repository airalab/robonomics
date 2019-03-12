use std::sync::Mutex;
use hyper::rt::Future;
use ipfs_api::IpfsClient;
use std::io::Write;
use futures::{Stream};
use std::fs::File;

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
pub fn read_file(ipfs_path: &'static str) -> std::io::Result<()> {

    let req = ipfs!().cat(ipfs_path)
        .concat2()
        .map(move |res| {
            let mut f = File::create(ipfs_path).expect("could not create file");
            f.write_all(&res).unwrap();
        })
    .map_err(|e| eprintln!("{}", e));
    hyper::rt::run(req);
    Ok(())
}