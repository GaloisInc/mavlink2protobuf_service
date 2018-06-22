extern crate mavlink;
extern crate protobuf;

use std::sync::Arc;
use std::thread;
use std::env;
use std::time::Duration;

use mavlink::common::*;

use std::io::prelude::*;
use std::net::TcpStream;

mod mavlink_common_gpb;
mod mavlink_connector;

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mavlink2protobuf (tcp|udpin|udpout|serial):(ip|dev):(port|baud) protobuf adr:port");
        return;
    }

    let vehicle = Arc::new(mavlink::connect(&args[1]).unwrap());
    //let mut stream = Arc::new(TcpStream::connect("127.0.0.1:34254").unwrap());

    // Transmit thread
    /*
    thread::spawn({
        let vehicle = vehicle.clone();
        move || loop {
            vehicle.send(&mavlink::heartbeat_message()).ok();
            thread::sleep(Duration::from_secs(1));
        }
    });
    */

    // Receive thread
    /*
    thread::spawn({
        let vehicle = vehicle.clone();
        //let stream = stream.clone();
        move || loop {
            if let Ok(msg) = vehicle.recv() {
                let protomsg = mavlink_connector::mavlink2protobuf(msg);
                let mavmsg = mavlink_connector::protobuf2mavlink(protomsg);
                println!("{:?}",mavmsg);
            }
        }
    });
    */
    loop {
        if let Ok(msg) = vehicle.recv() {
            let protomsg = mavlink_connector::mavlink2protobuf(msg);
            println!("{:?}", protomsg);
            let mavmsg = mavlink_connector::protobuf2mavlink(protomsg);
            println!("{:?}", mavmsg);
        }
    }
}
