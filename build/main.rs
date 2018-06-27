#![recursion_limit="128"]
#[macro_use]
extern crate quote;
extern crate byteorder;
extern crate crc16;
extern crate rustfmt;
extern crate xml;

mod parser;

use std::env;
use std::fs::File;
use std::path::Path;
use std::io::Write;
use std::process::Command;

pub fn main() {
    let _cmd = Command::new("mkdir").arg("protos").output().expect("command failed");

    // quote test
    let src_dir = env::current_dir().unwrap();
    let in_path = Path::new(&src_dir).join("common.xml");
    let mut inf = File::open(&in_path).unwrap();

    let src_dir = env::current_dir().unwrap();
    let dest_path_proto = Path::new(&src_dir).join("protos/mavlink_common.proto");
    let mut protof = File::create(&dest_path_proto).unwrap();

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path_rust = Path::new(&out_dir).join("common.rs");
    let mut rustf = File::create(&dest_path_rust).unwrap();

    parser::generate(&mut inf, &mut protof, &mut rustf);

    // format the protobuf file
    let cmd = Command::new("clang-format")
        .arg(&dest_path_proto.to_str().unwrap())
        .output()
        .expect("command failed");

    let mut outf = File::create(&dest_path_proto).unwrap();
    outf.write(&cmd.stdout);
}