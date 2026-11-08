//! 由 `.proto` 生成 prost 消息类型（ADR 0005 / protobuf-payload change）。
//!
//! 需要 `protoc` 在 PATH 或由 `PROTOC` 环境变量指定；CI 双平台须安装同版本
//! protoc 以保证生成结果一致（见 .github/workflows/ci.yml）。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/proto/w3net.proto");
    prost_build::compile_protos(&["src/proto/w3net.proto"], &["src/proto/"])?;
    Ok(())
}
