#![feature(extern_prelude)]
extern crate prost;
#[macro_use]
extern crate prost_derive;

extern crate byteorder;
extern crate crc16;
extern crate serial;

use std::io;
use byteorder::{ LittleEndian, ReadBytesExt, WriteBytesExt };
use std::io::prelude::*;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod connection;
pub use connection::{ MavConnection, Tcp, Udp, Serial, connect };

/// The MAVLink common message set
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/common.rs"));
}

use common::MavMessage;

const MAV_STX: u8 = 0xFE;

/// Metadata from a MAVLink packet header
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Header {
    pub sequence: u8,
    pub system_id: u8,
    pub component_id: u8,
}

/// Read a MAVLink message from a Read stream.
pub fn read<R: Read>(r: &mut R) -> io::Result<(Header, MavMessage)> {
    loop {
        if try!(r.read_u8()) != MAV_STX {
            continue;
        }
        let len    =  try!(r.read_u8()) as usize;
        let seq    =  try!(r.read_u8());
        let sysid  =  try!(r.read_u8());
        let compid =  try!(r.read_u8());
        let msgid  =  try!(r.read_u8());
        
        let mut payload_buf = [0; 255];
        let payload = &mut payload_buf[..len];
        try!(r.read_exact(payload));
        
        let crc = try!(r.read_u16::<LittleEndian>());

        let mut crc_calc = crc16::State::<crc16::MCRF4XX>::new();
        crc_calc.update(&[len as u8, seq, sysid, compid, msgid]);
        crc_calc.update(payload);
        crc_calc.update(&[MavMessage::extra_crc(msgid)]);
        if crc_calc.get() != crc {
            continue;
        }
        if let Some(msg) = MavMessage::parse(msgid, payload) {
            return Ok((Header { sequence: seq, system_id: sysid, component_id: compid }, msg));
        }
    }
}

/// Write a MAVLink message to a Write stream.
pub fn write<W: Write>(w: &mut W, header: Header, data: &MavMessage) -> io::Result<()> {
    let msgid = data.message_id();
    let payload = data.serialize();
    
    let header = &[
        MAV_STX,
        payload.len() as u8,
        header.sequence,
        header.system_id,
        header.component_id,
        msgid,
    ];
    
    let mut crc = crc16::State::<crc16::MCRF4XX>::new();
    crc.update(&header[1..]);
    crc.update(&payload[..]);
    crc.update(&[MavMessage::extra_crc(msgid)]);
    
    try!(w.write_all(header));
    try!(w.write_all(&payload[..]));
    try!(w.write_u16::<LittleEndian>(crc.get()));

    Ok(())
}