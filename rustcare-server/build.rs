fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "proto/healthcare.proto",
                "proto/auth.proto",
                "proto/workflow.proto",
                "proto/audit.proto",
            ],
            &["proto/"],
        )?;
    Ok(())
}