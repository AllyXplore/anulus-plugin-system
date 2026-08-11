# AxAIHub 插件体系（开源）

AxAIHub 的插件格式、打包工具与开发文档的开源仓库。

插件是 AxAIHub 的扩展机制：HTML 插件以独立窗口或 iframe 方式运行，通过 postMessage 桥调用宿主能力（存储 / 网络 / 剪贴板 / AI 对话 / 语音 / 通知 / 用户资料等），权限采用**声明 + 运行时授权**的严格模型。

## 仓库结构

```
├── docs/                     # 开发文档（规范 / 教程 / 架构 / API 参考）
│   ├── 01-插件格式规范.md    # .ns 元数据 + .axext/.axex 二进制格式 + 安全模型（权威）
│   ├── 02-打包教程.md        # 从零开发打包插件的完整教程
│   ├── 03-架构设计.md        # 插件体系模块划分 / 数据流 / 安全架构
│   └── 04-API参考.md         # 通信协议 + 可用能力清单 + AI 反向调用 / WASM / 办公页
├── tools/
│   ├── axbuild/              # 跨平台打包 CLI（Rust）：keypair / new / pack / info
│   └── README.md             # 工具总览
└── samples/
    ├── axplugin-sdk.js        # 插件参考 SDK（AX.call / AX.aiFunction / AX.on，零依赖）
    └── hello-plugin/          # 极简示例插件（可直接打包）
```

## 快速开始

```bash
# 1. 编译打包工具
cd tools/axbuild
cargo build --release

# 2. 生成签名密钥对（打包 .axex 加密包时用）
./target/release/axbuild keypair

# 3. 生成并打包一个插件
./target/release/axbuild new ./my-plugin --id com.example.demo --name "Demo" --author "me"
./target/release/axbuild pack ./my-plugin my-plugin.axext
```

把生成的 `.axext` / `.axex` 文件导入 AxAIHub（插件管理 → 导入）即可使用。

## 文档速览

- 想知道插件**能做什么、怎么通信** → `docs/04-API参考.md`
- 想知道**包格式和加密/签名细节** → `docs/01-插件格式规范.md`
- 想**从零写一个插件** → `docs/02-打包教程.md`

## 核心特性

- **两种分发格式**：`.axext` 明文 ZIP（开源分享）；`.axex` AES-256-GCM 加密 ZIP + 可选 Ed25519 作者签名（商业闭源，防源码泄露）
- **严格权限模型**：插件必须在 `.ns` 声明权限，用户运行时授权或管理页手动开启，未声明一律不可用
- **隔离运行**：插件跑在无同源的 iframe / 宿主页面中，通信只走 postMessage 桥；敏感命令（读文件 / 取密钥 / 终端等）在 Rust 层校验「仅主窗口可用」，插件一律够不到
- **AI 可反向调用（宿主端待接通）**：协议与插件侧实现就绪（`AX.aiFunction`，见 `docs/04-API参考.md` §4.2）；宿主 AI 到插件的投递链路当前存在缺陷，修复后生效
- **钱包保护**：`ai.chat` 由用户指定方案 + 可配置配额（分/日/月 + token 预算），API 密钥永不进入插件

## 许可证

- `docs/01-插件格式规范.md`（NS 语言与 `.axext` / `.axex` 格式规范）：**[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)**（`LICENSE-CC-BY-4.0.txt`）
- 其余内容（其他文档、`tools/axbuild` 打包工具、`samples` 示例）：**MIT**（`LICENSE`）

---

## 发布到 GitHub

本仓库已完成 `git init` 与初始提交，发布只需三步：

```bash
# 1. 设置你的 GitHub 身份（发布前必做，替换成你的信息）
git config user.name "你的GitHub用户名"
git config user.email "你的邮箱"

# 2. 在 github.com 新建一个空仓库（不要勾选 README/LICENSE/.gitignore，避免冲突），
#    复制仓库地址，例如 https://github.com/你的用户名/axaihub-plugin-system.git

# 3. 关联并推送
git remote add origin https://github.com/你的用户名/axaihub-plugin-system.git
git push -u origin main
```

推送完成后，GitHub 会自动渲染 `README.md` 和 `docs/` 下的 Markdown 文档，仓库即可公开访问。

> 可选：发布后可在仓库 Settings → Pages 中启用 GitHub Pages（Source 选 main 分支根目录），把文档站托管到 `https://你的用户名.github.io/axaihub-plugin-system/`。
