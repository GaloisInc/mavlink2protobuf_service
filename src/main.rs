extern crate mavlink;
extern crate protobuf;
extern crate protobuf_serde;

use std::env;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mavlink::common::*;

use std::io::prelude::*;
use std::net::TcpStream;

mod mavlink_common_gpb;
mod mavlink_connector;

use protobuf::Message;

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
    let mut m = mavlink_common_gpb::MavlinkMessage::new();
    let mut w = vec![];
    loop {
        if let Ok(msg) = vehicle.recv() {
            //println!("{:?}", msg);
            match msg {
                MavMessage::HEARTBEAT(data) => {
                    //println!("{:?}", data);
                    let mut v = vec![];
                    let mut n = mavlink_common_gpb::Heartbeat::new();
                    n.set_field_type(42);
                    n.set_autopilot(data.autopilot.into());
                    n.set_base_mode(data.base_mode.into());
                    n.set_custom_mode(data.custom_mode.into());
                    n.set_system_status(data.system_status.into());
                    n.set_mavlink_version(data.mavlink_version.into());
                    println!("has been initialized ={}",n.is_initialized());
                    
                    m.set_msg_id(12);
                    
                    //println!(">>> {}",serde_json::to_string(&m).unwrap());
                    println!("size = {}", n.compute_size());
                    //if v.len() < 16
                    {
                        let mut stream = protobuf::stream::CodedOutputStream::vec(&mut v);
                        n.write_to_with_cached_sizes(&mut stream).unwrap();
                    }
                    println!("n={:?}", n);
                    println!("v={:?}", v);
                    m.set_msg_data(v);

                    /*
                    let mut new_n = mavlink_common_gpb::Heartbeat::new();
                    {
                        let mut stream = protobuf::stream::CodedInputStream::from_bytes(&mut v[..]);
                        new_n.merge_from(&mut stream).unwrap();
                    }
                    println!("new n={:?}", new_n);
                    println!("v={:?}", v);
                    //return;
                    */
                    
                    
                    // big struct
                    {
                        let mut stream = protobuf::stream::CodedOutputStream::vec(&mut w);
                        m.write_to_with_cached_sizes(&mut stream).unwrap();
                    }
                    println!("m={:?}", m);
                    println!("w={:?}", w);

                    let mut new_m = mavlink_common_gpb::MavlinkMessage::new();
                    {
                        let mut stream = protobuf::stream::CodedInputStream::from_bytes(&mut w[..]);
                        new_m.merge_from(&mut stream).unwrap();
                    }
                    println!("new m={:?}", new_m);
                    println!("w={:?}", w);
                    
                    let mut new_n = mavlink_common_gpb::Heartbeat::new();
                    let mut data = new_m.take_msg_data();
                    {
                        let mut stream = protobuf::stream::CodedInputStream::from_bytes(&mut data[..]);
                        new_n.merge_from(&mut stream).unwrap();
                    }
                    println!("new n={:?}", new_n);
                    println!("v={:?}", data);
                     return;   
                }
                /*
                MavMessage::GLOBAL_POSITION_INT(data) => {
                    let mut n = mavlink_common_gpb::GlobalPositionInt::new();
                    n.set_time_boot_ms(data.time_boot_ms);
                    n.set_lat(data.lat);
                    n.set_lon(data.lon);
                    m.set_globalpositionint(n);
                    //println!("{:?}", m);
                    let mut v = vec![];
                    {
                        let mut stream = protobuf::stream::CodedOutputStream::vec(&mut v);
                        m.write_to_with_cached_sizes(&mut stream).unwrap();
                    }
                    //println!("v={:?}", v);
                }
                */
                _ => {}
            }
            /*
            println!("v.len={}", v.len());
            if v.len() > 10 {
                println!("m={:?}", m);
                println!("v={:?}", v);
                let mut new_m = mavlink_common_gpb::MavlinkMessage::new();
                {
                    let mut stream = protobuf::stream::CodedInputStream::from_bytes(&mut v[..]);
                    new_m.merge_from(&mut stream).unwrap();
                }
                println!("new_m={:?}, v.len={}", new_m, v.len());
                return;
            }
            */

            /*
            let protomsg = mavlink_connector::mavlink2protobuf(msg);
            println!("{:?}", protomsg);
            let mavmsg = mavlink_connector::protobuf2mavlink(protomsg);
            println!("{:?}", mavmsg);
            */
        }
    }
}
