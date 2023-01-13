fn main() -> Result<(), String> {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .compile(&["proto/hackernews_proxy.proto"], &["proto"])
        .map_err(|err| format!("Failed to compile proto: {err}!"))
}
