use std::sync::Mutex;
use rosrust::error::Result;
use rosrust::{
    api::{Ros, Topic, raii::{Publisher, Subscriber}},
    Message,
};

lazy_static! {
    static ref ROS: Mutex<Option<Ros>> = Mutex::new(None);
}

macro_rules! ros {
    () => {
        ROS.lock().unwrap().as_mut().unwrap()
    };
}

#[inline]
pub fn init() {
    let client = Some(Ros::new("robonomics").unwrap());
    let mut ros = ROS.lock().unwrap();
    *ros = client;
}

#[inline]
pub fn subscribe<T, F>(topic: &str, callback: F) -> Result<Subscriber>
where
    F: Fn(T) -> () + Send + 'static,
    T: Message
{
    ros!().subscribe::<T, F>(topic, callback)
}

#[inline]
pub fn publish<T>(topic: &str) -> Result<Publisher<T>>
where
    T: Message
{
    ros!().publish::<T>(topic)
}

#[inline]
pub fn topics() -> Vec<Topic> {
    ros!().topics().unwrap()
}