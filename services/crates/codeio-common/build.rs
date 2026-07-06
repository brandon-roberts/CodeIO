fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "../../../proto";

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "core/common.proto",
                "core/error.proto",
                "index/workspace_scan.proto",
                "index/context_index.proto",
                "index/dependency_map.proto",
                "ai/types.proto",
                "ai/spotlight.proto",
                "ai/context_window.proto",
                "ai/ai_service.proto",
                "frontend/ast.proto",
                "frontend/parse.proto",
                "frontend/typecheck.proto",
                "vm/vm_control.proto",
                "vm/execution.proto",
                "meta/macro.proto",
                "meta/dsl.proto",
            ],
            &[proto_root],
        )?;

    println!("cargo:rerun-if-changed={}", proto_root);
    Ok(())
}
