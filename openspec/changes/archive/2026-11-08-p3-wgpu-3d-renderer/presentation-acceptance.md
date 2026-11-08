# P3 呈现验收与残差记录

## 验收目标

按 `3d-rendering` spec 与 `design.md` 的 Risks 段，用 GPU readback / 截图验证
wgpu 呈现链路，并记录保真度与平台差异残差。呈现差异**不计入**模拟确定性
（ADR 0002）。

## 已验证（本会话，自动化）

- `w3-render` 全量测试通过：23 个库单测 + 3 个确定性隔离测试，
  `cargo test -p w3-render --all-targets` 全绿。
- `tests/isolation.rs`：模型加载失败、动画丢帧、位置插值、事件重复投递
  场景下，`compute_state_hash()` 与 `read_view()` 完全一致。
- GPU 代码路径可编译：窗口、交换链、渲染管线、离屏 readback 与
  `examples/render_frame.rs` 均通过 `cargo check`。
- PNG 编码器（零依赖 stored DEFLATE）、行对齐去除、BGRA→RGBA 交换、
  顶点生成可复现性均通过单元测试。
- `Scene::build_vertices` 使用 `BTreeMap` 稳定迭代，同输入逐帧结果一致，
  便于 readback 逐帧比对。

## 呈现验收实跑（人工本机，2026-11-08）

在 Windows 11 / rustc 1.91 桌面会话（Git Bash）执行：

```bash
cargo run --offline -p w3-render --example render_frame
```

产物 `target/w3-render-frame.png`：800×600 RGBA8、1,920,813 字节，PNG
chunk 为 IHDR / IDAT / IEND，解压后扫描线长度与尺寸严格吻合；由
`GpuContext::capture_rgba` 离屏 GPU readback 经零依赖 PNG 编码器写出，
`git check-ignore` 确认命中 `.gitignore:4:/target/`，未进入 git 跟踪。

像素核验（全图 480,000 像素）：背景 `(39,48,69)` 占 79.52%，地形
`(120,174,120)` 占 20.48%，alpha 全 255；中心行取样呈
「背景→地形→背景」。证明 clear、几何光栅化、着色、纹理拷贝与 PNG 编码
整条链路在真实 GPU 上跑通，而非纯清屏帧。

本次为**无参运行**：使用合成碗状高度场、模型库为空，单位按 spec
Scenario 降级为不显示，故截图中只有地形；合成地形顶点色 / 法线近似恒定，
地形区呈单一绿色。带本机导出资产的完整呈现仍建议按需复跑：

```bash
cargo run --offline -p w3-render --example render_frame -- <assets-dir> <output.png>
```

`<assets-dir>` 读 `terrain-heightfield.json` 与 `models/*.json`（其中
`models/default.json` 才会命中单位模型）；输出父目录须已存在，建议仍放
`target/` 或仓库外，避免 `git add -A` 误入库（ADR 0001）。

## 已知残差（对齐 design.md Risks）

- **wgpu 后端平台差异**：Vulkan / DX12 / Metal / GL 的呈现可能有像素级
  差异；用 readback 截图人工比对，但不把差异计入模拟确定性。
- **MDX 转换保真度**：当前只解析上游 `godot_warcraft3` 转换规格 JSON 子集
  （骨骼 / Sequence / GeosetAnim / FilterMode / 挂点），完整静态网格蒙皮与
  PE2 粒子留待后续；第一版用 CPU 展开的朝向标记占位呈现单位，非像素级对齐。
- **相机 / 色彩**：MVP 为按地形范围生成的固定视角，队伍色与地表色为近似；
  与 gamemd 的像素级对齐是 Non-Goal。
- **性能**：20,000 单位目标不在第一版承诺；当前为单 pipeline、非索引
  三角形列表，每帧重建顶点缓冲。
- **跨平台确定性**：模拟内核跨平台确定性仍为待验证项（ADR 0002），本
  change 的呈现验收不覆盖、也不声称该结论。

## 资产与法律

导出 JSON / PNG / GLB 等资产留在本机，`.gitignore` 与 CI 资产扫描双重把关；
本 change 未提交任何导出资产或截图（ADR 0001）。
