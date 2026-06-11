# 设计简报: Codex Lite

## 一句话描述

Codex Lite 是一个本地优先、无广告、轻量化的 Codex 账号管理工具，面向需要管理多个 Codex 账号的个人和小团队用户，解决账号导入、配额查看和一键切换的问题。

## 目标用户

- 主要用户: 先给小圈子朋友或团队使用，功能完善后开源发布到 GitHub 免费给更多用户使用。
- 用户特点: 重度使用 Codex 或 Codex CLI，有多个账号，需要频繁查看额度并快速切换当前账号；不希望工具混入广告、赞助推广或过重的平台矩阵。

## 核心场景

1. 用户打开工具，导入一个或多个 Codex 账号，并能看到账号列表。
2. 用户查看当前 Codex 账号、计划信息、Hourly/Weekly 配额和重置时间。
3. 用户选择某个账号，一键切换为本机 Codex 当前账号。

## 功能范围

### 第一版必须有

- Codex 账号列表管理。
- 从本机 `~/.codex/auth.json` 导入当前登录账号。
- JSON / 文件导入。
- 批量文件导入。
- OAuth 登录导入。
- Refresh Token / Token 导入。
- API Key 账号导入。
- 当前账号识别与展示。
- Codex 配额刷新与展示。
- 一键切换当前 Codex 账号。
- 基础账号删除、重命名、备注或标签管理。
- macOS / Windows 可用。
- 安装包构建能力。
- 自动检测 Codex 路径或本地配置路径。
- 基础日志和常见错误提示。

### 明确不做（留给后续版本）

- 广告、赞助位、推广页和公告营销系统。
- 多平台账号管理，例如 Cursor、GitHub Copilot、Windsurf、Kiro、Gemini CLI、Zed 等。
- 内置 CLIProxyAPI / API relay 服务，除非后续明确把 Codex API service 作为独立版本目标。
- 复杂多开实例管理。
- 悬浮窗、彩蛋、烟花动效等非核心体验。
- WebDAV 同步、复杂自动备份和云端同步。
- 18 种语言完整国际化。
- 复杂托盘菜单和平台布局自定义。

### 公开发布前补齐

- macOS / Windows / Linux 全平台可用。
- 完整 README 和使用说明。
- GitHub release 包。
- 更完整的错误恢复。
- 数据迁移机制。
- 更完善的日志与故障排查说明。

## 成功标准

- 第一阶段: 能让小圈子用户在 macOS / Windows 上安装使用，完成 Codex 账号导入、配额查看和一键切换；常见错误有明确提示，具备基础日志。
- 公开发布前: macOS / Windows / Linux 均可用，有完整 README、release 包、错误恢复和迁移机制，可以免费发布到 GitHub。

## 用户原话

> 我想根据他，做一个仿品，他现在这个太重了，而且有很多广告
>
> A
>
> 先B但是基本完善后，会公开发布到github，免费供大家使用
>
> A B C
>
> D
>
> 第一阶段做到 C，公开前再补到 D

## 项目现状

- 技术栈: 原项目是 Tauri 2 + Rust 后端 + React 19 + TypeScript + Vite 前端，使用 Zustand、i18next、Tailwind CSS、DaisyUI，并包含 Go sidecar `cockpit-cliproxy`。
- 项目结构: 前端位于 `src/`，Tauri/Rust 位于 `src-tauri/`，共享 Rust crates 位于 `crates/`，Go sidecar 位于 `sidecars/cockpit-cliproxy/`。
- 已有功能: 多平台账号管理、配额监控、Codex 账号管理、Codex API service、多平台实例管理、唤醒任务、设置、WebSocket、本地导入、OAuth、自动更新、托盘、悬浮卡片、备份同步、公告和赞助模块。
- 新功能定位: 不在原项目内继续堆功能，而是以原项目为参考，设计一个独立的 Codex-only 轻量产品，保留 Codex 核心能力，移除广告、多平台矩阵和非核心复杂功能。

## 补充信息

- 参考产品: Cockpit Tools。
- 用户偏好: 本地优先、干净无广告、免费开源、第一阶段先服务小圈子用户，后续公开发布 GitHub。
- 约束: 第一版只支持 Codex，不做其他 AI IDE 平台。
