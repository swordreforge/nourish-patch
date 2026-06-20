fn main() {
    println!("cargo:rerun-if-changed=proto/logs.proto");
    // Generate the gRPC client for the log stream (client only — the tool never serves).
    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&["proto/logs.proto"], &["proto"])
        .expect("compile logs.proto");

    // Tauri codegen (reads tauri.conf.json).
    tauri_build::build();
}
