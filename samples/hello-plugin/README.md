# 你好插件 (hello-plugin)

Anulus 插件最小示例：读取宿主信息 + 监听生命周期事件。

## 文件

- `plugin.ns` — NexusScript 元数据
- `index.html` — 插件界面

## 打包

```bash
# 需要 axbuild CLI, 见 tools/axbuild/
axbuild pack . hello.axext
```

## 说明

插件通过 `window.parent.postMessage` 与宿主通信（详见 `docs/04-API参考.md`）。示例为手写协议演示；实际开发建议直接用参考 SDK `samples/axplugin-sdk.js`（`AX.call` / `AX.on`）。

- `app.info`：读取宿主名称与版本（无需权限）
- `lifecycle` 事件：安装 / 启用 / 禁用 / 卸载时宿主自动派发（无需订阅，消息格式 `{type:'event', event:'lifecycle', payload:{type}}`）
