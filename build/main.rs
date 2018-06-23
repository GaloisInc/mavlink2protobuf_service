extern crate byteorder;
extern crate crc16;

//extern crate protobuf_codegen_pure;
extern crate protoc_rust;
extern crate xml;

mod parser;

use std::env;
use std::fs::File;
use std::path::Path;

use std::process::Command;

pub fn main() {
    let _cmd = Command::new("mkdir").arg("protos").output().expect("command failed");

    // Generate protobuf file from mavlink xml message description
    let src_dir = env::current_dir().unwrap();
    let in_path = Path::new(&src_dir).join("common.xml");
    let mut inf = File::open(&in_path).unwrap();

    let src_dir = env::current_dir().unwrap();
    let dest_path = Path::new(&src_dir).join("protos/mavlink_common.proto");
    let mut outf = File::create(&dest_path).unwrap();

    parser::generate_protobuf(&mut inf, &mut outf);

    // Generate rust protobuf implementation
    let src_dir = env::current_dir().unwrap();
    let in_path = Path::new(&src_dir).join("protos");
    //protobuf_codegen_pure::run(protobuf_codegen_pure::Args {
    protoc_rust::run(protoc_rust::Args {
        out_dir: &in_path.to_string_lossy(),
        input: &[
            &Path::new(&src_dir)
                .join("protos/mavlink_common.proto")
                .to_string_lossy(),
        ],
        includes: &[&in_path.to_string_lossy()],
        //customize: protobuf_codegen_pure::Customize {
        customize: protoc_rust::Customize {
            //serde_derive: Some(true),
            //carllerche_bytes_for_bytes: Some(true),
            //carllerche_bytes_for_string: Some(true),
            ..Default::default()
        },
    }).expect("protoc");

    let _cmd = Command::new("mv")
        .arg("protos/mavlink_common.rs")
        .arg("src/mavlink_common_gpb.rs")
        .output()
        .expect("command failed");

    // Generate mavlink<->protobuf conversion
    /*
    let src_dir = env::current_dir().unwrap();
    let in_path = Path::new(&src_dir).join("common.xml");
    let mut inf = File::open(&in_path).unwrap();

    let src_dir = env::current_dir().unwrap();
    let dest_path = Path::new(&src_dir).join("src/mavlink_connector.rs");
    let mut outf = File::create(&dest_path).unwrap();

    parser::generate_connector(&mut inf, &mut outf);
    */}
