# Security Policy

> English primary · 简体中文见下方分隔线

## Supported Versions

Only the latest released version of this repository is supported with security fixes.

| Version | Supported |
| ------- | --------- |
| v1.0.0  | ✅ |
| < v1.0.0| ❌ |

## Reporting a Vulnerability

**Please do NOT open a public issue for security vulnerabilities.**

Use one of these private channels:

1. **GitHub Private Vulnerability Reporting** (preferred): open the **Security** tab of this repo → **Report a vulnerability**. This keeps the report private until a fix is ready.
2. If that is unavailable, email the maintainer (see `README.md` → Author & Maintenance) and we will set up a private thread.

We aim to acknowledge reports within 7 days and provide a fix or mitigation plan within 30 days for confirmed issues.

## Scope notes (what this repo is / isn't)

- This repository ships the **plugin format, packaging CLI, and docs**. The Anulus app backend (Rust/JNI) lives in a separate private repository.
- `.axex` encryption (AES-256-GCM) and the import-time environment self-check (emulator / root / debugger detection) are **defensive measures, not a hard security boundary**. They slow down casual tampering; they are not hardware-level protection and can be bypassed by determined attackers. Do not rely on them to protect secrets.
- Plugins declare permissions in `.ns` and the user authorizes them. A malicious plugin can still abuse whatever permissions the user grants — review plugins before installing, especially closed-source `.axex` from untrusted authors.
- API keys for AI plans never enter plugins; `ai.chat` uses the user-selected plan and quota.

---

# 安全政策

> 中文版 · English above

## 受支持版本

仅本仓库最新发布版本提供安全修复。

| 版本 | 支持情况 |
| ---- | -------- |
| v1.0.0 | ✅ |
| < v1.0.0 | ❌ |

## 报告漏洞

**请勿公开提 Issue 报告安全漏洞。**

请使用以下私密渠道之一：

1. **GitHub 私有漏洞上报**（推荐）：进入本仓库 **Security** 标签页 → **Report a vulnerability**，报告在修复前保持私密。
2. 若不可用，发邮件给维护者（见 `README.md` → 作者与维护），我们另开私密沟通。

我们会在 7 天内确认收到，确认的漏洞会在 30 天内给出修复或缓解方案。

## 范围说明

- 本仓库仅包含**插件格式、打包工具与文档**。Anulus 应用后端（Rust/JNI）在独立私有仓库。
- `.axex` 加密与导入时的环境自检（模拟器 / Root / 调试检测）是**防御性措施，非硬性安全边界**，可被有经验的攻击者绕过，请勿用于保护机密。
- 插件在 `.ns` 声明权限、由用户授权；恶意插件仍可能滥用用户授予的权限，安装前请审阅，尤其来源不可信的闭源 `.axex`。
- AI 方案的密钥永不进入插件；`ai.chat` 使用用户指定方案与配额。
