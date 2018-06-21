extern crate crc16;
extern crate byteorder;
extern crate xml;

mod parser;

use std::env;
use std::fs::File;
use std::path::Path;

extern crate protobuf_codegen_pure;


pub fn main() {
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
    protobuf_codegen_pure::run(protobuf_codegen_pure::Args {
        out_dir: &in_path.to_string_lossy(),
        input: &[&Path::new(&src_dir).join("protos/mavlink_common.proto").to_string_lossy()],
        includes: &[&in_path.to_string_lossy()],
        customize: protobuf_codegen_pure::Customize {
            ..Default::default()
        },
    }).expect("protoc");
}
