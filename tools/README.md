# tools — 打包工具

| 工具 | 说明 | 位置 |
|------|------|------|
| `axbuild` | 跨平台命令行打包工具（Rust），产出 `.axext` / `.axex`，支持加密 + Ed25519 作者签名 | `tools/axbuild/` |
| AxExt 打包工具 | APP 内置打包页（手机端），产出与 axbuild 字节级对齐 | 见 `docs/04-API参考.md` 第 10 节 |

快速开始：

```bash
cd axbuild
cargo build --release
./target/release/axbuild keypair    # 生成签名密钥对
./target/release/axbuild new ./pkg --id com.example.demo --name Demo --author me
./target/release/axbuild pack ./pkg demo.axext
```
