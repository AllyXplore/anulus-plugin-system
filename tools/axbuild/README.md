# axbuild — AxAIHub 插件打包 CLI

跨平台命令行打包工具，产出与 APP 内 AxExt 打包工具**字节级对齐**的 `.axext` / `.axex` 插件包，两套工具生成的包可互相导入。

## 编译

需要 Rust 工具链（https://rustup.rs）：

```bash
cd tools/axbuild
cargo build --release
# 产物: target/release/axbuild(.exe)
```

## 使用

### 生成作者密钥对（首次必做，用于签名）

```bash
axbuild keypair
# -> 输出 私钥(base64, 保密) / 公钥(base64, 公开) / 指纹
```

### 生成插件模板

```bash
axbuild new ./my-plugin --id com.example.demo --name "Demo" --author "me"
# -> 生成 plugin.ns + index.html
```

### 打包为 .axext（开源明文格式）

```bash
axbuild pack ./my-plugin my-plugin.axext
```

### 打包为 .axex（加密，默认密钥）

```bash
axbuild pack ./my-plugin my-plugin.axex --encrypt
```

### 打包为 .axex（加密 + 作者签名，生产推荐）

```bash
axbuild pack ./my-plugin my-plugin.axex --encrypt --private-key "<私钥base64>"
```

### 查看包信息（含签名验证）

```bash
axbuild info my-plugin.axex
```

## 密钥说明

- 加密主密钥优先级：`--master-key` 参数 > 环境变量 `AXBUILD_MASTER_KEY` > 内置默认密钥 `AxAIHub-Default-Dev-Key!`（与 APP 内置一致）。
- 用**自定义密钥**加密的包，用户导入时必须在 APP 导入界面填写相同密钥才能解密；默认密钥包对用户透明。
- 作者签名（Ed25519）**没有默认密钥**，每位开发者必须用自己的私钥；签名用于防篡改 + 作者身份证明，详见 `docs/01-插件格式规范.md` 第 5 节。
