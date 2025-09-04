use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Compile Protocol Buffer definitions
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/daemon.proto"], &["proto"])?;
    
    println!("cargo:rerun-if-changed=proto/daemon.proto");
    
    Ok(())
}