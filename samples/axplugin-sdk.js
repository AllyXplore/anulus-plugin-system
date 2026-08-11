/* ============================================================================
 * axplugin-sdk.js — AxAIHub 插件开发参考 SDK
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
 *
 * 协议要点（与宿主严格一致）：
 *   - 请求:  {source:'axplugin', id, action, args} → 响应按 id 匹配
 *   - AI 调用: 宿主发 {type:'ai_call', function, args} → 插件回 ack → 心跳(2s) → result/error
 *   - 事件:  宿主发 {type:'event', event, payload}
 *
 * 注意：AX.aiFunction 的宿主投递链路已修复（AxPlugin.html 静默模式转发），随 APP 新版本生效；
 *       其余能力（AX.call / AX.on）在当前版本即可用。
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

  // 监听宿主事件（lifecycle / permission_result 等）
  function on(event, fn) {
    (listeners[event] = listeners[event] || []).push(fn);
  }

  function emit(event, payload) {
    (listeners[event] || []).forEach(function (fn) { try { fn(payload); } catch (e) {} });
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
    }, 2000);
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

  global.AX = {
    call: call,
    aiFunction: aiFunction,
    on: on
  };
})(window);
