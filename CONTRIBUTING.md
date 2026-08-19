# Contributing

> English primary · 简体中文见下方分隔线

Thanks for your interest in the AxAIHub plugin system.

## What can be contributed

- **Code, tools, samples, and non-spec docs** (`tools/axbuild`, `samples`, `docs/02`–`docs/04`, this file): licensed **MIT**, and pull requests are welcome.
- **`docs/01-插件格式规范.md` (the NS language & package format spec)**: licensed **CC BY 4.0**. You may adapt and redistribute it with attribution, but the canonical format is maintained by the author. Please **do not** open PRs that change the format itself — file an issue to discuss first. Changing the spec without coordination would fork compatibility.

## Development setup

```bash
# Build the packaging CLI
cd tools/axbuild
cargo build --release

# Smoke test
./target/release/axbuild new /tmp/demo --id com.example.demo --name Demo --author you
./target/release/axbuild pack /tmp/demo /tmp/demo.axext
./target/release/axbuild info /tmp/demo.axext
```

## Guidelines

- Keep `axbuild` dependency-free where possible; it must build on stable Rust.
- Match the postMessage protocol exactly as documented in `docs/04-API参考.md`. The host bridge is the source of truth.
- Write docs in Chinese (consistent with the app); keep code/comments clear.
- Open an issue before large changes so we can align.

---

# 贡献指南

> 中文版 · English above

感谢关注 AxAIHub 插件体系。

## 可以贡献什么

- **代码、工具、示例、非规范文档**（`tools/axbuild`、`samples`、`docs/02`–`docs/04`、本文件）：**MIT** 许可，欢迎提合并请求。
- **`docs/01-插件格式规范.md`（NS 语言与包格式规范）**：**CC BY 4.0**。你可署名改编与再分发，但规范由作者维护；**请勿**直接提改动格式本身的合并请求，请先提 Issue 讨论。擅自改动规范会造成兼容性分叉。

## 开发环境

```bash
cd tools/axbuild
cargo build --release
./target/release/axbuild new /tmp/demo --id com.example.demo --name Demo --author you
./target/release/axbuild pack /tmp/demo /tmp/demo.axext
./target/release/axbuild info /tmp/demo.axext
```

## 约定

- `axbuild` 尽量零依赖，须能在稳定版 Rust 编译。
- 严格遵循 `docs/04-API参考.md` 的 postMessage 协议，宿主桥为准。
- 文档用中文撰写（与 APP 一致），代码清晰。
- 大改动前先开 Issue 对齐。
