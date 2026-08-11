# AxAIHub 插件 API 参考

> 插件与宿主之间的完整通信协议、可用能力清单与扩展参考（AI 反向调用 / WASM / 办公页入口）。
> 格式与权限以 `01-插件格式规范.md` 为准，本文是面向插件开发者的能力与协议速查。

---

## 1. 插件包格式

插件包是一个 `.axext` / `.axex` 文件，本质为 ZIP 压缩包（`.axex` 为 AES-256-GCM 加密 ZIP）。用户导入后自动解压到应用的插件目录。

包内结构：

```
my-plugin.axext
├── plugin-id.ns    # 必需: NexusScript 元数据文件
├── index.html      # 必需: UI 入口 (local/wasm 类型)
├── logic.wasm      # 可选: WASM 后端逻辑
└── assets/         # 可选: CSS/JS/图片等资源
    └── style.css
```

> 网页链接 (remote) 类型插件无需打包文件，只需在管理页输入 URL。

---

## 2. 插件元数据字段 (.ns 格式)

| 字段 | 必填 | 说明 |
|------|------|------|
| `id` | 必填 | 插件唯一标识，仅允许字母数字下划线横线 |
| `name` | 必填 | 插件显示名称 |
| `version` | 必填 | 版本号，如 1.0.0 |
| `author` | 必填 | 作者名 |
| `description` | 可选 | 插件描述 |
| `type` | 必填 | `local`(本地HTML) / `wasm`(带WASM) / `remote`(网页链接) |
| `entry` | 必填 | 入口文件路径(local/wasm) 或 URL(remote) |
| `license` | 可选 | 版权声明，含 type/github/updateCheck |
| `permissions` | 可选 | 权限声明，见第 3 节 |
| `office` | 可选 | 办公页入口声明，见第 6 节 |

```ns
# plugin.ns
id          "my-plugin"
name        "我的插件"
version     "1.0.0"
author      "作者名"
description "插件描述"
type        local
entry       "index.html"

license {
    type        "MIT"
    github      "https://github.com/user/repo"
    updateCheck "https://raw.githubusercontent.com/user/repo/main/plugin.ns"
}
```

> 仅支持 `.ns` (NexusScript) 格式；旧 `manifest.json` 仅作导入回退，规范上不再使用。

---

## 3. 权限声明

在 `permissions` 对象中声明插件需要的权限。**严格模式：未声明的权限不可用**；声明后还需用户授权（运行时弹窗确认或插件管理页手动开启）。

| 权限 key | 说明 | 附加参数 |
|----------|------|----------|
| `storage` | 读写插件专属数据目录 | `quota`: 配额(如"10MB") |
| `network` | 发起网络请求(域名白名单) | `domains`: 域名数组, `rateLimit`: 次/分 |
| `clipboard` | 读写系统剪贴板 | - |
| `vibration` | 设备震动 | - |
| `tts` | 系统朗读 | - |
| `media` | 保存图片到相册 / 系统分享 | - |
| `notify` | 发送系统通知 | - |
| `ai` | AI 调用权限(可被 AI 反向调用) | `callable`: bool, `functions`: 函数数组 |
| `speech` | 系统语音识别 | - |
| `file` | 系统文件夹选择器 | - |
| `profile` | 读取宿主用户头像/昵称 (`user.profile`) | - |
| `office` | 办公页入口(在办公页显示按钮) | `enabled`, `title`, `desc`, `icon` |

```ns
# 权限声明示例 (.ns 写法)
permissions {
    storage { quota "10MB" }
    network { domains ["api.example.com"] rateLimit 10 }
    clipboard {}
    ai {
        callable true
        functions [
            {
                name        "calculate"
                description "计算数学表达式"
                params      [{ name "expression" type "string" }]
                returns     "number"
            }
        ]
    }
    profile {}
}
```

> 第 2 层系统权限（相机/麦克风/位置/通知/联系人/日历/短信/身体传感器/蓝牙/附近WiFi 等）由 APP 统一持有，插件通过 `permission.check` / `permission.request` **借用**，结果经 `permission_result` 事件异步回传。

---

## 4. 通信协议 (postMessage)

### 4.1 插件请求宿主能力

```javascript
// 插件 → 宿主: 请求
// 插件方用 '*' 作为 targetOrigin (插件无法预知宿主 origin)
window.parent.postMessage({
  source: 'axplugin',       // 固定
  id: 'req-001',            // 请求 ID, 用于匹配响应
  action: 'storage.read',   // 能力路径
  args: { filename: 'data.json' }
}, '*');

// 监听宿主响应
window.addEventListener('message', function(e) {
  if (e.data.source !== 'axhost') return;
  console.log(e.data.result || e.data.error);
});
```

### 4.2 宿主请求插件函数 (AI 调用)

声明了 `ai.callable: true` 的插件，宿主 AI 会按声明向插件发起函数调用：

```javascript
// 插件需监听此消息, 执行函数后返回结果
window.addEventListener('message', function(e) {
  if (e.data.source !== 'axhost') return;
  if (e.data.type === 'ai_call') {
    var callId = e.data.id;
    var funcName = e.data.function;
    var args = e.data.args;

    // 1. 先发送 ack (已收到)
    window.parent.postMessage({
      source: 'axplugin', id: callId, type: 'ack'
    }, '*');

    // 2. 耗时操作中定期发送心跳 (每 2 秒)
    var heartbeat = setInterval(function() {
      window.parent.postMessage({
        source: 'axplugin', id: callId, type: 'heartbeat'
      }, '*');
    }, 2000);

    // 3. 执行函数
    var result = myFunction(args);

    // 4. 返回结果
    clearInterval(heartbeat);
    window.parent.postMessage({
      source: 'axplugin', id: callId,
      type: 'result', result: result
    }, '*');
  }
});
```

> 心跳协议：宿主等待 500ms 内收到 ack，否则判定无响应；处理期间每 3 秒需发送 heartbeat，否则判定卡死；总超时 30 秒（可配置）。

---

## 5. 可用 API 列表

| action | 说明 | 所需权限(第1层) |
|--------|------|----------|
| `app.info` | 获取应用信息 | 无 |
| `app.openUrl` | 在系统浏览器打开链接（仅 http/https） | 无 |
| `app.share` | 系统分享面板分享图片 | media |
| `storage.read` / `storage.write` | 读写插件私有数据文件 | storage |
| `storage.list` / `storage.delete` | 列举 / 删除插件数据文件 | storage |
| `network.fetch` | 经 APP 代理发起网络请求（域名白名单+限频） | network |
| `clipboard.read` / `clipboard.write` | 读写剪贴板 | clipboard |
| `device.vibrate` | 触发设备震动（`{duration?}` 毫秒，上限 5000） | vibration |
| `tts.speak` | 系统朗读文本（`{text, rate?, pitch?}`） | tts |
| `media.save` | 保存图片到系统相册（`{dataUrl, mediaType?}`） | media |
| `notify.show` | 发送系统通知（`{title, body}`，需 APP 已获通知权限） | notify |
| `ai.chat` | 调用宿主 AI 对话（`{messages, profileName?}`，支持多模态传图；方案由用户指定，插件无法绕过） | ai |
| `api.catalog` | 查看 APP 的 API 方案目录（脱敏：名称/模型/能力，绝不返回密钥） | 无 |
| `speech.recognize` / `speech.stop` | 系统语音识别（结果经 `speech_result` 事件异步回传 `{text, error}`） | speech |
| `file.pickFolder` | 系统文件夹选择器（SAF，阻塞轮询，返回 `{uri, name}`） | file |
| `user.profile` | 读取宿主用户头像/昵称（返回 `{granted, name, avatar}`，未授权时浮层弹授权框） | profile |
| `permission.request` | 申请：能力权限弹宿主确认框；系统权限走 APP 通道 | 见上 |
| `permission.check` | 查询：能力授权状态 / APP 系统权限状态 | 见上 |
| `event.subscribe` / `event.unsubscribe` | 订阅 / 取消事件 | 无 |
| `lifecycle` 事件 | 安装/启用/禁用/卸载时自动派发（无需订阅） | 无 |
| `permission_result` 事件 | 系统权限申请结果异步回传（`{permType, granted, requestCode}`） | 无 |

### 完整调用示例

```javascript
function callHost(action, args) {
  return new Promise(function(resolve, reject) {
    var id = 'req-' + Date.now();
    var handler = function(e) {
      if (e.data.source !== 'axhost' || e.data.id !== id) return;
      window.removeEventListener('message', handler);
      if (e.data.error) reject(e.data.error);
      else resolve(e.data.result);
    };
    window.addEventListener('message', handler);
    window.parent.postMessage({
      source: 'axplugin', id: id,
      action: action, args: args || {}
    }, '*');
  });
}

// 使用示例
callHost('storage.write', { filename: 'data.json', dataB64: btoa('{"a":1}') })
  .then(function(path) { console.log('已保存到', path); });
```

> `network.fetch` 的 body 需 Base64 编码后通过 `bodyB64` 传递。宿主会校验请求域名是否在 `.ns` 的 `network.domains` 白名单内，并按 `rateLimit`(次/分) 做滑动窗口频率限制。

---

## 6. 办公页入口声明

插件可以声明在办公页显示一个按钮，点击后打开插件的页面。页面风格完全自定义，无需与主应用统一。

```ns
# .ns 中声明 office 字段
office {
    enabled true
    title   "我的工具"
    desc    "工具描述"
    icon    "<svg viewBox=...></svg>"
}
```

> 用户需在插件管理中授权「办公页入口」权限后，按钮才会显示。点击按钮跳转到 AxPlugin.html 全屏运行插件。

---

## 7. AI 调用插件

声明了 `ai.callable: true` 的插件可以被 AI 通过工具调用系统调用。

### 7.1 AI 工具调用格式

```
[TOOL:plugin]插件id|函数名|JSON参数[/TOOL]

# 示例
[TOOL:plugin]my-calculator|calculate|{"expression":"1+2*3"}[/TOOL]
```

### 7.2 函数声明格式

```json
{
  "name": "calculate",          // 函数名
  "description": "执行计算",    // 描述(给 AI 看)
  "params": [                   // 参数列表
    { "name": "expression", "type": "string" }
  ],
  "returns": "number",          // 返回类型
  "timeout": 30                 // 超时秒数(默认30)
}
```

AI 收到的可用函数列表格式：

```
- my-calculator.calculate(expression: string): 执行计算
- my-dict.lookup(word: string): 查词典
```

---

## 8. WASM 插件

WASM 插件本质是「HTML 外壳 + WASM 逻辑」。HTML 负责 UI 渲染，WASM 负责计算密集型逻辑。

```html
<!-- index.html 中加载 WASM -->
<script>
WebAssembly.instantiateStreaming(fetch('logic.wasm'))
  .then(function(obj) {
    var wasm = obj.instance.exports;
    var result = wasm.myFunction(42);
    console.log(result);
  });
</script>
```

> WASM 文件需与 index.html 一起打包到 `.axext` 中。WASM 插件与普通 HTML 插件使用相同的通信协议和权限系统。

---

## 9. 完整示例：带 AI 调用功能的计算器插件

```ns
# plugin.ns
id          "calc-demo"
name        "计算器Demo"
version     "1.0.0"
author      "Demo"
type        local
entry       "index.html"

permissions {
    storage {
        quota "5MB"
    }
    ai {
        callable true
        functions [
            {
                name        "calculate"
                description "计算数学表达式"
                params      [{ name "expression" type "string" }]
                returns     "number"
            }
        ]
    }
}
```

```html
<!-- index.html -->
<!DOCTYPE html>
<html>
<body>
  <input type="text" id="expr" placeholder="1+2*3">
  <button onclick="doCalc()">计算</button>
  <div id="result"></div>
  <script>
    function calc(expr) {
      try { return eval(expr); } catch(e) { return 'Error'; }
    }
    function doCalc() {
      document.getElementById('result').textContent =
        calc(document.getElementById('expr').value);
    }
    // 监听 AI 调用
    window.addEventListener('message', function(e) {
      if (e.data.source !== 'axhost' || e.data.type !== 'ai_call') return;
      var id = e.data.id;
      window.parent.postMessage({source:'axplugin', id:id, type:'ack'}, '*');
      var result = calc(e.data.args.expression || '');
      window.parent.postMessage({
        source:'axplugin', id:id, type:'result', result: result
      }, '*');
    });
  </script>
</body>
</html>
```

---

## 10. 打包

- **CLI**（跨平台，推荐生产环境）：见 `tools/axbuild/`，支持 `keypair` / `new` / `pack` / `info` 四个命令。
- **手机端**：APP 内 AxExt 打包工具，产出与 CLI 字节级对齐，两套工具生成的 `.axex` 可互相导入。
- 简单场景也可直接打 ZIP：`zip -r my-plugin.axext plugin.ns index.html assets/`
