extern crate mavlink_proto;
extern crate zmq;
#[macro_use]
extern crate clap;

#[cfg(feature = "json")]
extern crate serde_json;

use std::sync::Arc;
use std::thread;

use clap::App;

use mavlink_proto::common::*;

/// Run with for example `cargo run -- udpin:127.0.0.1:14540 tcp://127.0.0.1:4441 tcp://127.0.0.1:44440`
fn main() {
    // The YAML file is found relative to the current file, similar to how modules are found
    let yaml = load_yaml!("../../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let device = matches.value_of("MAVLINK_DEVICE").unwrap();
    println!("Mavlink connecting to {}", device);
    let vehicle = Arc::new(mavlink_proto::connect(device).unwrap());
    let context = zmq::Context::new();

    thread::spawn({
        let vehicle = vehicle.clone();
        let subscriber = context.socket(zmq::SUB).unwrap();
        let filter = "";
        let addr = matches.value_of("ADDR_SUB").unwrap();

        match subscriber.connect(addr) {
            Ok(_) => {
                println!("Subscriber: connected to {}", addr);
            }
            Err(e) => {
                println!("Subscriber error: {} connecting to {}", e, addr);
                exit(1);
            }
        }
        assert!(subscriber.set_subscribe(filter.as_bytes()).is_ok());

        move || loop {
            #[cfg(feature = "json")]
            {
                // For JSON data
                let stream = subscriber.recv_string(0).unwrap().unwrap();
                println!("Received msg = {}", stream);
                let msg: MavMessage = serde_json::from_str(&stream).unwrap();
                vehicle.send(&msg).ok();
            }

            #[cfg(not(feature = "json"))]
            {
                let stream = subscriber.recv_bytes(0).unwrap();
                println!("Received {} bytes", stream.len());
                let msg = MavMessage::from_proto_msg(stream).unwrap();
                vehicle.send(&msg).unwrap();
            }
        }
    });

    // TX thread
    let publisher = context.socket(zmq::PUB).unwrap();
    let addr = matches.value_of("ADDR_PUB").unwrap();
    match publisher.bind(addr) {
        Ok(_) => {
            println!("Publisher: bound to {}", addr);
        }
        Err(e) => {
            println!("Publisher error: {} connecting to {}", e, addr);
            exit(1);
        }
    }

    loop {
        if let Ok(msg) = vehicle.recv() {
            if matches.is_present("debug") {
                println!("{:?}", msg);
            }

            #[cfg(not(feature = "json"))]
            publisher.send(&msg.encode(), 0).unwrap(); // send &w with 0 flags

            #[cfg(feature = "json")]
            {
                // For JSON data
                let stream = serde_json::to_string(&msg).unwrap();
                println!("Sending msg = {}", stream);
                publisher.send(stream.as_bytes(), 0).unwrap(); // send &w with 0 flags
            }
        }
    }
}
