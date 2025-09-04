use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // Get the path to the protos directory
    let proto_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?)
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
        .build_server(false)  // Only need client
        .build_client(true)
        .out_dir("src/generated")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute(".", "#[serde(rename_all = \"snake_case\")]")
        .compile(&[proto_file], &[proto_dir])?;
    
    println!("Generated gRPC client code from daemon_api.v1.proto");
    
    Ok(())
}