# CLAUDE.md — Toyomimi

> 🚩 **新会话第一件事：读 `docs/toyomimi/00-context.md`**（现状、已定决策、文档地图）。
> 本仓是**公开仓库**：这里只写工程规则，不写任何个人信息、账号、秘钥。

## 这是什么

**Toyomimi（豊聡耳）** —— 轻量桌面端实时翻译：采集**系统声音**（可选麦克风），经云端同传引擎翻成「我的语言」，悬浮字幕显示。
两个场景：**多人跨语言在线会议**、**看视频**。

本仓 **fork 自 [phuc-nt/my-translator](https://github.com/phuc-nt/my-translator)**（MIT，v0.9.1 / 2026-07-11）。
原作者署名与 `LICENSE` 必须保留。通用改进可以回馈上游（`upstream` remote）。

「要不要做、为什么」在 PKB `10-Projects/Vibe Coding/vibe-Toyomimi/`；「是什么、怎么跑」在本仓 `docs/toyomimi/`。同一句话只写一处，冲突时以本仓为准。

## 技术栈（继承上游）

- **Tauri 2**：Rust 后端 `src-tauri/`，原生 JS 前端 `src/`（无框架、无打包器）
- 音频：macOS ScreenCaptureKit（`src-tauri/src/audio/system_audio.rs`）· Windows WASAPI loopback（`wasapi.rs`）→ 16kHz PCM
- 引擎：Soniox · OpenAI Realtime · Qwen LiveTranslate · 本地 MLX（前端 `src/js/*-client.js` / `soniox.js`，后端 `src-tauri/src/commands/`）

## 常用命令

```bash
npm install
npm run dev          # 开发（读 .env 里的 APP_IDENTIFIER，让 dev 版有独立的 macOS 权限）
npm run dev:web      # 只看前端 UI（http://localhost:3111，Tauri API 走 tauri-mock.js）
npm run build        # 打包
npm run lint         # cargo clippy
```

前置：Rust 工具链（`rustup`）+ Xcode Command Line Tools。

## 铁律

1. **秘钥只进 `.env`（已 gitignore）或 App 设置界面**，绝不写进代码、文档、commit。本仓公开，泄露即作废——换 Key，别指望删 commit。
2. **提交前必看 `git status`**：确认 `.env`、`local/` 没进暂存区。私有测试音频、会议录音一律放 `local/`。
3. **不动上游的 `LICENSE` 与原作者署名。** 我们的新增说明写在 README 顶部的 Toyomimi 区块和 `docs/toyomimi/`。
4. **身份已改为 Toyomimi**（`io.github.nexustbh.toyomimi`，更新端点与公钥都指向我们）。**不要改回、不要从上游合并覆盖 `tauri.conf.json` 的这些字段**；从 `upstream` 合并时逐项核对。
5. **Mac + Windows 通用是选 Tauri 的原因**：先把 Mac 做好，但**每次提交都要让 Windows 能编译**（CI `ci.yml` 会查）。平台相关代码只能写在 `#[cfg(target_os = ...)]` 里，跨平台地图见 `docs/toyomimi/00-context.md` §3。
6. **引擎可插拔是核心设计**：新引擎按现有 client 的事件形状接入（原文/译文 × 临时/最终、说话人、用量、错误），不在 UI 层写引擎特判。
7. **批量脚本用 `python3` 或 node**，不用 bash 关联数组。
8. **测量先于结论**：引擎延迟 / 质量 / 成本的任何说法，要有 `scripts/phase0/` 的实测数据或注明「厂商宣称」。

## 协作约定

- 维护者不写代码：你给方案（2–3 个选项 + 代价 + 推荐），他回决策
- 界面改动截图给他看，不让他读代码
- 一个阶段一个 commit；**push 与发版由维护者决定**
- 做完一段：`docs/toyomimi/Build-Log.md` 追加一行；踩坑写 `docs/toyomimi/bugs.md`；需要拍板的写进 PKB 决策记录
