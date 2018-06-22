#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate mavlink;
extern crate protobuf;
extern crate protobuf_serde;

//extern crate bytes;

use std::sync::Arc;
use std::thread;
use std::env;
use std::time::Duration;

use mavlink::common::*;

use std::io::prelude::*;
use std::net::TcpStream;

mod mavlink_common_gpb;
//mod mavlink_connector;

fn main() {
    /*
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: mavlink2protobuf (tcp|udpin|udpout|serial):(ip|dev):(port|baud) protobuf adr:port");
        return;
    }
    let vehicle = Arc::new(mavlink::connect(&args[1]).unwrap());
    */

    let vehicle = Arc::new(mavlink::connect("udpin:127.0.0.1:14550").unwrap());
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
            //println!("{:?}", msg);
            let mut m = mavlink_common_gpb::MavlinkMessage::new();
            if let MavMessage::HEARTBEAT(data) = msg {
              //println!("{:?}", data);
              let mut n = mavlink_common_gpb::MavlinkMessage_Heartbeat::new();
              n.set_field_type(42);
              n.set_autopilot(data.autopilot.into());
              n.set_base_mode(data.base_mode.into());
              n.set_custom_mode(data.custom_mode.into());
              n.set_system_status(data.system_status.into());
              n.set_mavlink_version(data.mavlink_version.into());
              m.set_heartbeat(n);
              println!(">>> {}",serde_json::to_string(&m).unwrap());
            }
            
            /*
            let protomsg = mavlink_connector::mavlink2protobuf(msg);
            println!("{:?}", protomsg);
            let mavmsg = mavlink_connector::protobuf2mavlink(protomsg);
            println!("{:?}", mavmsg);
            */
        }
    }
}
