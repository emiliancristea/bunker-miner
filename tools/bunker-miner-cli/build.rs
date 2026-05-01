use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    env::set_var("PROTOC", protoc);

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let generated_dir = manifest_dir.join("src").join("generated");
    fs::create_dir_all(&generated_dir)?;

    // Get the path to the protos directory
    let proto_dir = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("protos");

    let proto_file = proto_dir.join("daemon_api.v1.proto");

    println!("cargo:rerun-if-changed={}", proto_file.display());
    println!("cargo:rerun-if-changed=build.rs");

    // Configure tonic-build for gRPC client generation
    tonic_build::configure()
        .build_server(false) // Only need client
        .build_client(true)
        .out_dir(generated_dir)
        .compile(&[proto_file], &[proto_dir])?;

    println!("Generated gRPC client code from daemon_api.v1.proto");

    Ok(())
}
