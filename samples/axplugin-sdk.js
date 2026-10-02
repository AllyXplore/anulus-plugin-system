/* ============================================================================
 * axplugin-sdk.js — Anulus 插件开发参考 SDK
 * ----------------------------------------------------------------------------
 * 轻量、零依赖：封装与宿主通信的 postMessage 协议（见 docs/04-API参考.md）。
 *
 * 用法：
 *   在插件 index.html 中 <script src="axplugin-sdk.js"></script>（随包一起打包）
 *
 * 能力：
 *   AX.call(action, args, timeoutMs?)  调用宿主能力（如 AX.call('storage.read', {filename:'a.json'})）
 *   AX.aiFunction(name, fn)           注册可被宿主 AI 反向调用的函数（需 .ns 声明 ai.callable）
 *   AX.on(event, fn)                  监听宿主事件（lifecycle / permission_result 等）
 *   AX.aiApi.register(key, name, model, desc?)  注册对外 AI 接口（需 .ns 声明 ai_api 权限）
 *   AX.aiApi.setVisible(key, visible)           控制接口是否显示给用户（随时可切）
 *   AX.aiApi.remove(key)                        注销接口
 *   AX.onAiApiRequest(handler)        接收宿主对话请求，handler({messages, model}) 返回文本或 Promise<文本>
 *                                     （用户在模型选择器选中你的接口后，对话请求会回发到这里）
 *
 * 协议要点（与宿主严格一致）：
 *   - 请求:  {source:'axplugin', id, action, args} → 响应按 id 匹配
 *   - AI 调用: 宿主发 {type:'ai_call', function, args} → 插件回 ack → 心跳(3s) → result/error
 *   - AI 接口请求: 宿主发 {type:'ai_api_request', payload:{messages, model}} → 插件回 ack → 心跳(3s)
 *                  → {type:'ai_api_response', ok:true, content} 或 {ok:false, error}
 *   - 事件:  宿主发 {type:'event', event, payload}
 *
 * 注意：AX.aiFunction 的宿主投递链路已接通（AxPlugin.html 静默模式转发）；
 *       其余能力（AX.call / AX.on）同样可用。
 *
 * 设备能力（需先 permission.request 获取系统权限，未授权时动作返回 need_permission）：
 *   AX.call('camera.capture', {})         拍照 → 结果经事件 'camera_result' 回传 {success, mime, data(图片base64)|error}
 *   AX.call('mic.record', {durationMs:10}) 录音(1-30秒) → 结果经事件 'mic_result' 回传 {success, mime, data(aac base64)|error}
 *   AX.call('geo.get', {})                 定位 → 同步返回 {lat, lng, accuracy, provider}，无定位返回 null
 *   示例：AX.on('camera_result', fn) 监听拍照结果（事件名与 04-API参考.md 一致）
 * ========================================================================== */
(function (global) {
  'use strict';

  var seq = 0;
  var pending = {};       // id -> {resolve, reject, timer}
  var aiFunctions = {};   // 可被 AI 调用的函数表
  var listeners = {};     // 事件监听器

  // 调用宿主能力；默认 5 秒超时
  function call(action, args, timeoutMs) {
    timeoutMs = timeoutMs || 5000;
    return new Promise(function (resolve, reject) {
      var id = 'req-' + (++seq);
      var timer = setTimeout(function () {
        if (pending[id]) { delete pending[id]; reject(new Error('宿主请求超时: ' + action)); }
      }, timeoutMs);
      pending[id] = { resolve: resolve, reject: reject, timer: timer };
      window.parent.postMessage({ source: 'axplugin', id: id, action: action, args: args || {} }, '*');
    });
  }

  // 注册可被宿主 AI 调用的函数（需 .ns 声明 ai { callable true functions [...] }）
  function aiFunction(name, fn) {
    aiFunctions[name] = fn;
  }

  // ---------------- 对外 AI 接口（需 .ns 声明 ai_api 权限） ----------------
  // 你的插件作为"算力提供方"向用户开放 AI 接口：
  //   1) 启动后调用 AX.aiApi.register(...) 注册（同 key 重复调用即更新）
  //   2) 用户在模型选择器里看到它（灰色字会标注你的插件名），选中后对话请求回发给插件
  //   3) 随时 AX.aiApi.setVisible(key, false) 可把接口从选择器里隐藏
  //   4) 宿主把组装好的消息数组发给你，你自行请求服务器后把完整回复文本交回
  var aiApiHandlers = [];   // 收到对话请求时的处理器

  function aiApiRegister(key, name, model, description) {
    return call('aiapi.register', { profileKey: key, name: name, model: model, description: description || '' }, 10000);
  }
  function aiApiSetVisible(key, visible) {
    return call('aiapi.setVisible', { profileKey: key, visible: !!visible }, 10000);
  }
  function aiApiRemove(key) {
    return call('aiapi.remove', { profileKey: key }, 10000);
  }
  function onAiApiRequest(handler) {
    if (typeof handler === 'function') aiApiHandlers.push(handler);
  }

  // AI 接口请求协议: ack → 心跳(3s) → ai_api_response
  function handleAiApiRequest(msg) {
    var callId = msg.id;
    var payload = msg.payload || {};
    window.parent.postMessage({ source: 'axplugin', id: callId, type: 'ack' }, '*');
    var heartbeat = setInterval(function () {
      window.parent.postMessage({ source: 'axplugin', id: callId, type: 'heartbeat' }, '*');
    }, 3000);
    var sendOk = function (content) {
      clearInterval(heartbeat);
      window.parent.postMessage({ source: 'axplugin', id: callId, type: 'ai_api_response', ok: true, content: String(content == null ? '' : content) }, '*');
    };
    var sendErr = function (err) {
      clearInterval(heartbeat);
      window.parent.postMessage({ source: 'axplugin', id: callId, type: 'ai_api_response', ok: false, error: String(err) }, '*');
    };
    if (!aiApiHandlers.length) { sendErr('插件未注册请求处理器(AX.onAiApiRequest)'); return; }
    (function run(i) {
      if (i >= aiApiHandlers.length) { sendErr('插件未处理该请求'); return; }
      try {
        var r = aiApiHandlers[i](payload);
        if (r && typeof r.then === 'function') { r.then(sendOk, sendErr); }
        else sendOk(r);
      } catch (err) { sendErr(err); }
    })(0);
  }

  // 监听宿主事件（lifecycle / permission_result 等）
  function on(event, fn) {
    (listeners[event] = listeners[event] || []).push(fn);
  }

  function emit(event, payload) {
    (listeners[event] || []).forEach(function (fn) { try { fn(payload); } catch (e) { console.warn('[axplugin-sdk] 事件回调出错:', e); } });
  }

  // 宿主消息分发
  window.addEventListener('message', function (e) {
    if (!e.data || e.data.source !== 'axhost') return;

    // 1) 请求响应（按 id 匹配）
    var h = pending[e.data.id];
    if (h) {
      clearTimeout(h.timer);
      delete pending[e.data.id];
      if (e.data.error) h.reject(new Error(e.data.error));
      else h.resolve(e.data.result);
      return;
    }

    // 2) AI 反向调用（宿主 → 插件）
    if (e.data.type === 'ai_call') {
      handleAiCall(e.data);
      return;
    }

    // 2.5) 对外 AI 接口请求（宿主 → 插件）
    if (e.data.type === 'ai_api_request') {
      handleAiApiRequest(e.data);
      return;
    }

    // 3) 事件（lifecycle / permission_result / ...）
    if (e.data.type === 'event' && e.data.event) {
      emit(e.data.event, e.data.payload);
      return;
    }
    if (e.data.type === 'permission_result') {
      emit('permission_result', e.data.payload || e.data);
    }
  });

  // AI 调用协议: ack → 心跳(2s) → result / error
  function handleAiCall(msg) {
    var callId = msg.id;
    var func = aiFunctions[msg.function];
    window.parent.postMessage({ source: 'axplugin', id: callId, type: 'ack' }, '*');
    if (typeof func !== 'function') {
      window.parent.postMessage({
        source: 'axplugin', id: callId, type: 'result',
        result: '[未注册函数: ' + msg.function + ']'
      }, '*');
      return;
    }
    var heartbeat = setInterval(function () {
      window.parent.postMessage({ source: 'axplugin', id: callId, type: 'heartbeat' }, '*');
    }, 3000); // 心跳间隔 3 秒，与 docs/04-API参考.md 一致
    var sendResult = function (result) {
      clearInterval(heartbeat);
      window.parent.postMessage({ source: 'axplugin', id: callId, type: 'result', result: result }, '*');
    };
    var sendError = function (err) {
      clearInterval(heartbeat);
      window.parent.postMessage({ source: 'axplugin', id: callId, type: 'error', error: String(err) }, '*');
    };
    try {
      var r = func(msg.args || {});
      if (r && typeof r.then === 'function') { r.then(sendResult, sendError); }
      else sendResult(r);
    } catch (err) {
      sendError(err);
    }
  }

  // ---- 宿主环境 (免权限公开信息, 2026-09-30 新增) ----
  // 界面语言来自主应用自身的语言设置 (不是系统语言): 插件据此自动匹配界面语言,
  // 无需再询问用户选择语言。语言码形如 zh-CN / en-US。
  var _envLang = null;
  function envGetLanguage() {
    if (_envLang) return Promise.resolve(_envLang);
    return call('env.get', {}).then(function (r) {
      _envLang = (r && r.language) || 'zh-CN';
      return _envLang;
    });
  }
  // 启动即预取: 多数场景下 AX.env.language 直接可用
  envGetLanguage().catch(function () {});

  global.AX = {
    call: call,
    aiFunction: aiFunction,
    on: on,
    // 对外 AI 接口提供 (此前版本函数已定义但漏挂到 AX 上, 属缺陷, 2026-09-30 修复)
    aiApi: {
      register: aiApiRegister,
      setVisible: aiApiSetVisible,
      remove: aiApiRemove
    },
    onAiApiRequest: onAiApiRequest,
    env: {
      // 异步取语言 (推荐): 首次调用走一次桥, 之后直接返回缓存
      getLanguage: envGetLanguage,
      // 同步读 (启动后短暂为 null, 预取完成后有值; 不确定时机就用 getLanguage())
      get language() { return _envLang; }
    }
  };
})(window);
