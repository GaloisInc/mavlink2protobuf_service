#![feature(extern_prelude)]
extern crate mavlink;

extern crate byteorder; // 1.2.3




extern crate protobuf;
//extern crate protobuf_serde;
extern crate zmq;

use std::env;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mavlink::common::*;

use std::io::prelude::*;
use std::net::TcpStream;

//mod mavlink_common_proto;
mod mavlink_connector;

use protobuf::Message;

fn main() {
    let vehicle = Arc::new(mavlink::connect("udpin:127.0.0.1:14550").unwrap());

    let context = zmq::Context::new();
    let publisher = context.socket(zmq::PUB).unwrap();

    assert!(publisher.bind("tcp://*:5556").is_ok());
    assert!(publisher.bind("ipc://weather.ipc").is_ok());

    loop {
        if let Ok(msg) = vehicle.recv() {
            let proto = mavlink_connector::mavlink2protobuf(msg);
            let mut stream = vec![];
            {
                let mut s = protobuf::stream::CodedOutputStream::vec(&mut stream);
                proto.write_to_with_cached_sizes(&mut s).unwrap();
            }
            println!("m={:?}", proto);
            println!("w={:?}", stream);
            publisher.send(&stream, 0).unwrap(); // send &w with 0 flags
        }
    }
}
