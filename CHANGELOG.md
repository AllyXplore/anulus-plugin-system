# Changelog

## v1.1.0 (2026-09-30)

### 文档
- 格式规范新增 §2.4「清单字段约束」：安装扫描器强制执行的字段硬约束（`name` ≤ 40 字、`author` ≤ 30 字、`description` 兜底 ≤ 2000 字符等）
- 说明 `description`（应用介绍）上限由 200 字放宽至 2000 字符兜底：英文 200 字符（含空格）不够写；安装确认页介绍区改为内部滚动展示，长介绍不再影响安装
- README（中/英）新增「安装时安全扫描」特性说明：结构校验、未知权限拒绝、恶意启发式检测、风险红字、高危禁装
- SDK 新增插件对外 AI 接口（`AX.aiApi.*` / `AX.onAiApiRequest`）与语言识别（`AX.env.getLanguage()`），详见 `docs/04-API参考.md`

## v1.0.0 (2026-08-18)

### 初始发布
- 插件格式规范：NS 元数据语言、.axext 明文格式、.axex 加密格式（AES-256-GCM + 可选 Ed25519 签名）
- 打包工具 axbuild（Rust CLI）：keypair / new / pack / info 命令
- 开发文档：格式规范、打包教程、架构设计、API 参考
- 示例项目：hello-plugin 最小示例 + axplugin-sdk.js 参考 SDK
- CI/CD：GitHub Actions 构建与冒烟测试
- Issue/PR 模板、贡献指南、安全政策

### Bug 修复
- 修复 NS 解析器引号键名宽容处理不生效的问题（strip_quotes 辅助函数）
- 统一 SDK 心跳间隔为 3 秒，与 API 参考文档一致
- SDK 事件回调错误不再静默吞掉，改为 console.warn 提示
- 添加 plugin_id 字符集校验，防止非法 ID 生成无效包