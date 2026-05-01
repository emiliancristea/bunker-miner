use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    env::set_var("PROTOC", protoc);

    // Compile Protocol Buffer definitions
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/daemon.proto"], &["proto"])?;
    
    println!("cargo:rerun-if-changed=proto/daemon.proto");
    
    Ok(())
}
