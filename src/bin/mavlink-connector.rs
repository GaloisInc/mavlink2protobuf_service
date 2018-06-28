extern crate zmq;
extern crate mavlink_proto;

//extern crate serde;
//extern crate serde_json;

use std::sync::Arc;
use std::thread;
use std::env;
use std::time::Duration;

use mavlink_proto::common::*;

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mavlink-connector (tcp|udpin|udpout|serial):(ip|dev):(port|baud)");
        return;
    }

    let vehicle = Arc::new(mavlink_proto::connect(&args[1]).unwrap());

    vehicle.send(&mavlink_proto::request_parameters()).unwrap();
    vehicle.send(&mavlink_proto::request_stream()).unwrap();

    thread::spawn({
        let vehicle = vehicle.clone();
        move || {
            loop {
                vehicle.send(&mavlink_proto::heartbeat_message()).ok();
                thread::sleep(Duration::from_secs(1));
            }
        }
    });

    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB).unwrap();

    assert!(publisher.bind("tcp://*:5556").is_ok());
    assert!(publisher.bind("ipc://weather.ipc").is_ok());

    loop {
        if let Ok(msg) = vehicle.recv() {
            //println!("{:?}", msg);
            // MavMessage::parse()
            match msg {
                MavMessage::SYS_STATUS(data) => {
                    println!("{:?}", data);
                    let stream = data.write_to_protostream().unwrap();
                    println!("stream.len={}",stream.len());
                    
                    publisher.send(&stream, 0).unwrap(); // send &w with 0 flags        
                }
                _ => {}
            }
        }
    }
}