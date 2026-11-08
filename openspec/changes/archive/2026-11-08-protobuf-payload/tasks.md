## 1. 依赖与合规前置

- [x] 1.1 解决 `prost` / `prost-build` 依赖供给（本机缓存与镜像连通性），确定并锁定版本
- [x] 1.2 按 CODEBUDDY.md §6 检查 Apache-2.0 与 GPLv3 兼容性，在 `NOTICE.md` 登记新增依赖并提交 `Cargo.lock`
- [ ] 1.3 为 `w3-net` 增加 `build.rs` / `.proto` 生成流程，验证 Windows / Linux 双平台生成结果字节一致

## 2. Schema 与编解码

- [x] 2.1 定义 `.proto`：`CommandBatch` / `Command`（`oneof` 覆盖全部 `CommandKind`）/ `Welcome` / `Ack` / `Checksum` / `ResyncRequest` / `ResyncReplay` / `Reject`
- [x] 2.2 用 `prost` 替换 `encode_commands` / `decode_commands` 及控制包 payload 构造与解析
- [x] 2.3 保持 `W3N1` 25B 帧头与 `Packet::encode` / `decode` 头逻辑不变，确认 `payload_len` 与 FNV-1a 覆盖 protobuf 编码后字节
- [x] 2.4 落实 canonical 约束：禁 `map`、固定字段编号与 `reserved`、禁 unknown fields 透传、禁非规范 varint、禁 `float` / `double`、`SimFixed` 用 `raw()` i64、schema 变更同步 bump `PROTOCOL_VERSION`

## 3. 确定性回归

- [ ] 3.1 补齐 protobuf 编解码 roundtrip 与 canonical 字节一致性单测（含双平台生成结果比对）
- [x] 3.2 重跑 `cargo test -p w3-core --test determinism` 作为基线校验（w3-core 零改动）
- [x] 3.3 双客户端逐帧哈希一致与丢包 / 乱序 / 短暂断线回归用例全绿
- [x] 3.4 真实 UDP 双进程实跑 30 帧，两端 `last_hash` 一致

## 4. 门禁与文档

- [x] 4.1 在归档 `vera20k-mapping.md` 追加 protobuf 演进注记，撤销「零新 crates.io 依赖」的现状表述
- [x] 4.2 `cargo fmt --all -- --check`、`cargo test --workspace`、`python tools/verify_all.py`、`openspec validate protobuf-payload` 全绿
- [ ] 4.3 CI Windows / Linux 双平台 `verify_all.py` 通过
