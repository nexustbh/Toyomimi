# Toyomimi — 新会话上下文

> 新会话先读本文。更新于 2026-09-22。

## 1. 现状

- 2026-09-22 从 `phuc-nt/my-translator` v0.9.1 fork，**代码尚未改动**，仍是上游原样。
- 当前阶段：**第 0 阶段 · 验证**（见 [`PLAN.md`](PLAN.md)）。

## 2. 已定决策（浓缩；带理由的完整版在 PKB `vibe-toyomimi-decisions.md`）

| # | 决策 | 结论 |
|---|---|---|
| D1 | 项目名 | **Toyomimi（豊聡耳）**：圣德太子能同时听十人陈情并逐一听懂 → 多人会议、各说母语 |
| D2 | 起步方式 | **fork my-translator**（MIT），不从零写 |
| D3 | 仓库 | `nexustbh/Toyomimi`，**public** |
| D4 | 技术栈 | Tauri 2（继承上游），Mac + Windows 一套代码 |
| D5 | 会议形态 | **对称部署**：每人的 App 只把「扬声器声音」翻成「我的语言」，双方无需互通协议；麦克风通道可选 |
| D6 | 主力引擎 | Qwen3.8-LiveTranslate（国内、中英、说话人区分）；Soniox 为对照组；引擎保持可插拔 |

## 3. 文档地图

| 文件 | 内容 |
|---|---|
| `CLAUDE.md` | 工程铁律 |
| `docs/toyomimi/PLAN.md` | 分阶段计划与验收标准 |
| `docs/toyomimi/Build-Log.md` | 编年史（只追加） |
| `docs/toyomimi/bugs.md` | 踩坑 |
| `scripts/phase0/` | 引擎实测脚本（第 0 阶段产出） |
| `local/` | 私有测试音频、录音（gitignore） |
| 其余 `docs/*` | **上游原文档**，参考用 |

## 4. 硬事实

| 项 | 值 |
|---|---|
| 上游 | `upstream` → `https://github.com/phuc-nt/my-translator.git`，fork 点 `3495f99`（v0.9.1） |
| 我方远端 | `origin` → `https://github.com/nexustbh/Toyomimi.git` |
| 上游身份（待改） | `productName: MyTranslator` · `identifier: com.personal.translator` · updater 指向上游 releases |
| 上游 Qwen 接入 | `src/js/qwen-realtime-client.js` + `src-tauri/src/commands/qwen_realtime.rs`，接的是**上一代** LiveTranslate Flash |
