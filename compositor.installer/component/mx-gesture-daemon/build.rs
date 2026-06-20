fn main() -> Result<(), Box<dyn std::error::Error>> {
    // This tells cargo to re-run this build script ONLY if the .proto file changes.
    println!("cargo:rerun-if-changed=./navigator.proto"); // cite: I changed the name here so the println log successfuly triggers cargo rerun

    tonic_prost_build::configure()
        // If you want to derive specific traits like serde::Serialize, you can do it here
        // .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["./navigator.proto"], &["./"])?;

    Ok(())
}
