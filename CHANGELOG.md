# 变更日志

所有重要变更都会记录在此文件。

## [0.1.0] - 首次发布

初始开源版本，包含：

- **文档**：插件格式规范（.ns 语言 + .axext/.axex 二进制格式 + 安全模型）、打包教程、架构设计、API 参考
- **工具**：axbuild 命令行打包工具（keypair / new / pack / info，支持 AES-256-GCM 加密与 Ed25519 作者签名）
- **示例**：参考 SDK（axplugin-sdk.js）与极简示例插件（hello-plugin）

### 已接通能力

app.info / app.openUrl / app.share / storage.* / network.fetch / clipboard.* / device.vibrate / tts.speak / media.save / notify.show / ai.chat / api.catalog / speech.* / file.pickFolder / user.profile / permission.* / event.* / lifecycle 事件 / permission_result 事件 / AI 反向调用（ai_call）

### 规划中能力

camera.capture / mic.record / geo.get（系统权限通道已就绪，宿主桥待实现）
