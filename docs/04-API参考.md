# Anulus 插件 API 参考

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
| `file` | 插件数据目录文件操作（读写/建目录/复制/移动/重命名/删除/搜索），以及系统文件/文件夹选择器 | - |
| `profile` | 读取宿主用户头像/昵称 (`user.profile`) | - |
| `ai_api` | 对外提供 AI 接口（接收对话内容并转发给插件服务器） | - |
| ~~`office`~~ | **已移除**（2026-09-26）：办公页入口功能整体下线，声明无效 | - |

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

> 不想手写协议？可直接使用参考 SDK：`samples/axplugin-sdk.js`（`AX.call` / `AX.aiFunction` / `AX.on` / `AX.aiApi.*` / `AX.onAiApiRequest`，零依赖，与本文协议严格一致）。

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

> **状态：已接通。** 宿主经隐藏的静默模式加载 AxPlugin.html 与插件通信（插件侧实现见参考 SDK 的 `AX.aiFunction`）。

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

> 心跳协议：宿主等待 ack（约 5 秒）判定插件是否响应；处理期间每 3 秒需收到 heartbeat，否则判定卡死；总超时 30 秒（可配置）。

---

## 5. 可用 API 列表

| action | 说明 | 所需权限(第1层) |
|--------|------|----------|
| `app.info` | 获取应用信息 | 无 |
| `env.get` | 获取宿主环境信息，当前含 `language`（主应用界面语言，如 zh-CN / en-US）。插件应据此自动匹配界面语言，无需询问用户；SDK 直接用 `AX.env.getLanguage()` | 无（免权限公开信息） |
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
| `aiapi.register` | 注册/更新本插件对外提供的 AI 接口（`{profileKey, name, model, description?}`；同 profileKey 覆盖更新） | ai_api |
| `aiapi.setVisible` | 控制本插件接口是否显示给用户（`{profileKey, visible}`，随时可切） | ai_api |
| `aiapi.remove` | 注销本插件注册的接口（`{profileKey}`） | ai_api |
| `file.mkdir` / `file.copy` / `file.move` / `file.rename` | 建目录 / 复制 / 移动 / 重命名（均限定在插件数据目录内） | file |
| `file.delete` | 删除文件或目录（不可撤销，宿主弹窗向用户确认一次） | file |
| `file.search` / `file.info` | 按名称递归搜索（`{dir?, query?, limit?}`）/ 查询大小与时间（`{path}`） | file |
| `ble.scan` / `ble.connect` / `ble.send` / `ble.notify` | 蓝牙扫描 / 连接 / 发送 / 订阅通知（可连接任意蓝牙设备，敏感） | bluetooth |
| `ble.pair` / `ble.disconnect` | 系统配对 / 断开 | bluetooth |
| `ui.confirm` / `ui.alert` / `ui.toast` | 调用宿主同款弹窗（宿主渲染，永远在插件内容之上，不可被遮挡或仿冒） | 无 |
| `api.catalog` | 查看 APP 的 API 方案目录（脱敏：名称/模型/能力，绝不返回密钥） | 无 |
| `speech.recognize` / `speech.stop` | 系统语音识别（结果经 `speech_result` 事件异步回传 `{text, error}`） | speech |
| `file.pickFolder` | 系统文件夹选择器（SAF，阻塞轮询，返回 `{uri, name}`） | file |
| `camera.capture` | 拍照（调系统相机，结果经 `camera_result` 事件异步回传 `{success, mime, data(图片base64) \| error}`） | camera（系统权限） |
| `mic.record` | 录音（`{durationMs}` 秒，1-30 默认 10，结果经 `mic_result` 事件异步回传 `{success, mime, data(aac base64) \| error}`） | microphone（系统权限） |
| `geo.get` | 获取最近定位（同步返回 `{lat, lng, accuracy, provider}`；无定位返回 `null`） | location（系统权限） |
| `user.profile` | 读取宿主用户头像/昵称（返回 `{granted, name, avatar}`，未授权时浮层弹授权框） | profile |
| `permission.request` | 申请：能力权限弹宿主确认框；系统权限走 APP 通道 | 见上 |
| `permission.check` | 查询：能力授权状态 / APP 系统权限状态 | 见上 |
| `event.subscribe` / `event.unsubscribe` | 订阅 / 取消事件 | 无 |
| `lifecycle` 事件 | 安装/启用/禁用/卸载时自动派发（无需订阅） | 无 |
| `permission_result` 事件 | 系统权限申请结果异步回传（`{permType, granted, requestCode}`） | 无 |
| `camera_result` 事件 | 拍照结果异步回传（`{success, mime, data \| error}`，需先授权系统相机权限） | 无 |
| `mic_result` 事件 | 录音结果异步回传（`{success, mime, data \| error}`，需先授权系统麦克风权限） | 无 |

### 事件订阅

| 事件 | 说明 |
|------|------|
| `lifecycle` | 安装/启用/禁用/卸载时自动派发（无需订阅） |
| `permission_result` | 系统权限申请结果异步回传（`{permType, granted, requestCode}`） |
| `camera_result` / `mic_result` / `speech_result` | 拍照 / 录音 / 语音识别结果异步回传 |

> 事件经 `event.subscribe` 订阅（`lifecycle` 无需订阅）。相机/麦克风/定位**能力动作**（`camera.capture` / `mic.record` / `geo.get`）已接通：使用前需先 `permission.request` 获取对应**系统权限**（弹系统授权窗），未授权时动作返回 `{granted:false, status:'need_permission', perm:'camera'|'microphone'|'location'}`。电池优化豁免、无障碍为 APP 级专属，插件声明无效。

### 对外 AI 接口（插件作为算力提供方）

用户不必部署服务或手输 API 密钥：下载算力提供方的官方插件即可获得 AI 接口。插件在 `.ns` 中声明 `ai_api` 权限并注册接口，用户在主应用模型选择器里选中后，对话请求回发给插件，由插件自行连接服务器取回数据。宿主不保存插件的端点与密钥。

展示规则（硬性约定）：插件提供的接口在模型选择器中以**灰色字标注来源插件名**，与用户自配方案明确区分；插件可随时用 `aiapi.setVisible` 控制自己的接口是否显示。

```javascript
// 1) 插件启动时注册接口（同 profileKey 重复调用即更新；需 .ns 声明 ai_api 权限）
await AX.aiApi.register('main', '算力方模型', 'model-x / 快速', '一句话说明');
//    用户看到的名字是 name；模型选择器中会灰字显示 "· 来自插件 <你的插件名>"

// 2) 随时控制显示/隐藏（例如账户额度用尽时先隐藏）
await AX.aiApi.setVisible('main', false);
await AX.aiApi.setVisible('main', true);

// 3) 接收对话请求: handler 收到 {messages, model}，返回文本（或 Promise<文本>）
AX.onAiApiRequest(async function (payload) {
  // payload.messages 为宿主组装好的消息数组 [{role, content}, ...]
  const reply = await fetchYourServer(payload.messages); // 你自己发网络请求
  return reply;                                          // 返回完整回复文本
});

// 4) 注销接口
await AX.aiApi.remove('main');
```

协议细节（SDK 已封装，手写时参考）：宿主发 `{source:'axhost', id, type:'ai_api_request', payload:{messages, model}}`；插件先回 `{source:'axplugin', id, type:'ack'}`，处理期间每 3 秒回 `{type:'heartbeat'}`（宿主按心跳判活：空闲 15 秒 / 总超时 180 秒），完成后回 `{source:'axplugin', id, type:'ai_api_response', ok:true, content}` 或 `{ok:false, error}`。插件卸载时其注册的接口会被宿主一并清除。

### 设备能力调用示例（相机 / 录音 / 定位）

```javascript
// 1) 先申请系统权限 (弹系统授权窗, 结果经 permission_result 事件回传)
await AX.call('permission.request', { permission: 'camera' });

// 2) 拍照: 结果经 camera_result 事件异步回传 {success, mime, data(图片base64)|error}
AX.on('camera_result', function(res) {
  if (res.success) { /* res.data 为 JPEG base64, 可直接 <img src="data:image/jpeg;base64,..."> */ }
});
await AX.call('camera.capture', {});   // 未授权时返回 {granted:false, status:'need_permission'}

// 3) 录音 10 秒: 结果经 mic_result 事件异步回传 {success, mime, data(aac base64)|error}
AX.on('mic_result', function(res) {
  if (res.success) { /* res.data 为 AAC base64 */ }
});
await AX.call('mic.record', { durationMs: 10 });

// 4) 定位: 同步返回 {lat, lng, accuracy, provider}; 无定位返回 null
const geo = await AX.call('geo.get', {});
if (geo) { console.log('经度', geo.lng, '纬度', geo.lat); }
```

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

> **状态：已接通**（同 §4.2）：插件侧声明与 SDK 就绪，宿主可调用插件声明的 AI 函数。

声明了 `ai.callable: true` 的插件可以被 AI 通过工具调用系统调用。

### 7.1 AI 工具调用格式

```
[TOOL:plugin]插件id|函数编号|JSON参数[/TOOL]

# 说明
- 每个插件声明的 AI 可调用函数由 **APP 统一分配全局唯一编号**（`P1` / `P2` / `P3` …，按插件 id 字典序 + 函数声明顺序分配）。
- 开发者**无需自定义编号、也无需保证函数名全局唯一**（系统只要求单插件内函数名不重复）；AI 收到的"可用插件函数"清单里是「编号 + 说明文本」。
- 这样从根上避免跨插件同名函数导致的调用歧义，AI 只需按编号调用。

# 示例（编号由 APP 分配，此处假设 calculate 分配为 P1）
[TOOL:plugin]my-calculator|P1|{"expression":"1+2*3"}[/TOOL]

# 兼容
历史消息 / 旧版本中的函数名写法（`[TOOL:plugin]my-calculator|calculate|...`）仍会被 APP 解析，新调用请优先使用编号。
```

### 7.2 函数声明格式

```json
{
  "name": "calculate",          // 函数名(仅内部标识, 调用统一用系统分配的编号)
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
    // 安全表达式求值: 纯算术解析器 (只支持数字 + - * / ( ) 与小数点)
    // 不执行任意代码 —— 不要用 eval / new Function, 用户输入会被当作脚本执行
    function calc(expr) {
      var s = String(expr || '').replace(/\s+/g, '');
      if (!/^[0-9+\-*/().]+$/.test(s) || s === '') return 'Error';
      var pos = 0;
      function peek() { return s[pos]; }
      function next() { return s[pos++]; }
      // 语法: expr = term (('+'|'-') term)* ; term = factor (('*'|'/') factor)* ; factor = number | '(' expr ')'
      function parseExpr() {
        var v = parseTerm();
        while (peek() === '+' || peek() === '-') {
          var op = next();
          var r = parseTerm();
          v = op === '+' ? v + r : v - r;
        }
        return v;
      }
      function parseTerm() {
        var v = parseFactor();
        while (peek() === '*' || peek() === '/') {
          var op = next();
          var r = parseFactor();
          if (op === '/' && r === 0) throw new Error('div0');
          v = op === '*' ? v * r : v / r;
        }
        return v;
      }
      function parseFactor() {
        if (peek() === '(') { next(); var v = parseExpr(); if (next() !== ')') throw new Error('paren'); return v; }
        var start = pos, dots = 0;
        while (/[0-9.]/.test(peek() || '')) {
          if (peek() === '.') { dots++; if (dots > 1) throw new Error('num'); }
          next();
        }
        if (start === pos) throw new Error('num');
        return parseFloat(s.slice(start, pos));
      }
      try { return parseExpr(); } catch (e) { return 'Error'; }
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
