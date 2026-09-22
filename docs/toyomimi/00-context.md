# Toyomimi — 新会话上下文

> 新会话先读本文。更新于 2026-09-22。

## 1. 现状

- fork 自 `phuc-nt/my-translator` v0.9.1，已完成**改名**（名称 / identifier / 版本 / 自动更新 / 数据目录），功能代码未改。
- Mac 本机可构建、可启动（`Toyomimi Dev.app`）。
- 2026-09-22 用测试脚本验证：**Qwen3.8（北京）连接与翻译正常**；Soniox Key 有效但账户余额为 0。App 内的 Qwen 仍是上游旧接入，**还不能直接用我们的北京 Key**（bugs B5）。
- CI（`ci.yml`）每次 push 在 **macOS + Windows** 双平台做 `cargo check`。
- 当前阶段：**第 0 阶段 · 验证**（见 [`PLAN.md`](PLAN.md)）。

## 2. 已定决策（浓缩；带理由的完整版在 PKB `vibe-toyomimi-decisions.md`）

| # | 决策 | 结论 |
|---|---|---|
| D1 | 项目名 | **Toyomimi（豊聡耳）** |
| D2 | 起步方式 | **fork my-translator**（MIT），不从零写 |
| D3 | 仓库 | `nexustbh/Toyomimi`，**public** |
| D4 | 平台 | **Mac + Windows 通用**是选 Tauri 的核心原因；**先把 Mac 做到正常可用，再做 Windows**，但每次提交都要保证 Windows 能编译 |
| D5 | 会议形态 | **对称部署**：每人的 App 只把扬声器声音翻成「我的语言」，双方零协议；麦克风可选 |
| D6 | 主力引擎 | Qwen3.8-LiveTranslate；Soniox 对照；引擎可插拔 |
| D8 | 身份 | identifier `io.github.nexustbh.toyomimi`；版本从 `0.10.0` 起（接上游 0.9.1，避免与上游 tag 撞名） |

## 3. 跨平台地图（Windows 移植时从这里开始）

| 层 | macOS | Windows | 共用 |
|---|---|---|---|
| 系统声音 | `src-tauri/src/audio/system_audio.rs`（ScreenCaptureKit，需「录屏」权限） | `src-tauri/src/audio/wasapi.rs`（WASAPI loopback） | `audio/mod.rs` 统一导出 `SystemAudioCapture` |
| 麦克风 / 重采样 | — | — | `microphone.rs`（cpal）· `resampler.rs`（→ 16kHz mono s16le） |
| 引擎 / UI / 设置 | — | — | 全部共用 |
| 本地 MLX 引擎 | 仅 Apple Silicon | ❌ | — |
| 签名 | 暂 ad-hoc（`-`）；正式分发需 Apple Developer ID | 暂不签名（SmartScreen 会提示） | 自动更新签名：Tauri minisign 密钥 |
| 打包 | `.app` / `.dmg`（arm64 + x64） | NSIS `.exe` / `.msi` | `release.yml`（打 `v*` tag 触发，三平台并行） |

**规矩**：平台相关代码只能出现在 `#[cfg(target_os = ...)]` 分支里，上层逻辑不感知平台。

## 4. 文档地图

| 文件 | 内容 |
|---|---|
| `CLAUDE.md` | 工程铁律 |
| `docs/toyomimi/PLAN.md` | 分阶段计划与验收标准 |
| `docs/toyomimi/Build-Log.md` | 编年史（只追加） |
| `docs/toyomimi/bugs.md` | 踩坑 |
| `scripts/phase0/` | 引擎实测脚本（第 0 阶段产出） |
| `local/` | 私有测试音频、录音（gitignore） |
| 其余 `docs/*`、`benchmarks/` | **上游原文档**，参考用 |

## 5. 硬事实

| 项 | 值 |
|---|---|
| 远端 | `origin` = `nexustbh/Toyomimi`（gh 默认仓库已设为它）· `upstream` = `phuc-nt/my-translator`，fork 点 `3495f99` |
| 身份 | `productName: Toyomimi` · `identifier: io.github.nexustbh.toyomimi` · dev 版 `…toyomimi.dev`（名为「Toyomimi Dev」） |
| 设置文件 | `~/Library/Application Support/io.github.nexustbh.toyomimi/settings.json` |
| 自动更新 | 端点 `github.com/nexustbh/Toyomimi/releases/latest/download/latest.json`；私钥 `~/.tauri/toyomimi.key`（**仓外、无密码、需备份**），公钥已写入 `tauri.conf.json` |
| 工具链 | Rust 1.98.1（rustup，含 `x86_64-apple-darwin`）· node 本机 `~/.local/bin/node` · Xcode CLT |
| 首次 Rust 构建 | 约 2 分钟；增量快得多 |
| Qwen3.8 接入（已实测） | `wss://dashscope.aliyuncs.com/api-ws/v1/realtime?model=qwen3.8-livetranslate-flash-realtime`，头 `Authorization: Bearer` + `X-DashScope-WorkSpace`；协议见 bugs B5 与 `scripts/phase0/probe.py` |
| 引擎实测 | `uv run --with websockets scripts/phase0/probe.py <音频> --engines qwen,soniox --seconds 60 --target zh` |

## 6. 常用命令

```bash
source ~/.cargo/env                 # 新 shell 里 cargo 不在 PATH 时
npm run build:dev                   # 构建 Toyomimi Dev.app（读 .env 的 APP_IDENTIFIER，ad-hoc 签名，带 DevTools）
open "src-tauri/target/release/bundle/macos/Toyomimi Dev.app"
npm run dev                         # 热重载开发（同样走 dev identifier）
cd src-tauri && cargo check         # 快速编译检查
gh run list --limit 5               # 看 CI
```

⚠️ ad-hoc 签名每次重新构建都会变，macOS 会**重新要录屏权限**（见 bugs B2）。
