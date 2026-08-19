# Changelog

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