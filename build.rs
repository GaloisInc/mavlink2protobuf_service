extern crate protobuf_codegen_pure;

fn main() {
    protobuf_codegen_pure::run(protobuf_codegen_pure::Args {
        out_dir: "/home/michal/Workspace/CPS/mavlink2protobuf_service/protos",
        input: &["/home/michal/Workspace/CPS/mavlink2protobuf_service/protos/mavlink_common.proto"],
        includes: &["/home/michal/Workspace/CPS/mavlink2protobuf_service/protos"],
        customize: protobuf_codegen_pure::Customize {
            ..Default::default()
        },
    }).expect("protoc");
}
