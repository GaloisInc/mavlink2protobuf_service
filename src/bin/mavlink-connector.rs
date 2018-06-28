extern crate mavlink_proto;
extern crate zmq;
extern crate serde_json;

use std::env;
use std::sync::Arc;
use std::thread;

use mavlink_proto::common::*;

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mavlink-connector (tcp|udpin|udpout|serial):(ip|dev):(port|baud)");
        return;
    }

    let vehicle = Arc::new(mavlink_proto::connect(&args[1]).unwrap());
    let context = zmq::Context::new();

    thread::spawn({
        let vehicle = vehicle.clone();
        let subscriber = context.socket(zmq::SUB).unwrap();
        let filter = "";
        assert!(subscriber.connect("tcp://localhost:5555").is_ok());
        assert!(subscriber.set_subscribe(filter.as_bytes()).is_ok());
        println!("Waiting for data...");

        move || loop {
            let stream = subscriber.recv_string(0).unwrap().unwrap();
            println!("Received msg = {}",stream);
            let msg: MavMessage = serde_json::from_str(&stream).unwrap();
            vehicle.send(&msg).ok();
        }
    });

    // TX thread
    let publisher = context.socket(zmq::PUB).unwrap();
    assert!(publisher.bind("tcp://*:5556").is_ok());
    assert!(publisher.bind("ipc://mavlink.ipc").is_ok());
    loop {
        if let Ok(msg) = vehicle.recv() {
            let stream = serde_json::to_string(&msg).unwrap();
            println!("Sending msg = {}",stream);
            publisher.send(stream.as_bytes(), 0).unwrap(); // send &w with 0 flags
        }
    }
}
