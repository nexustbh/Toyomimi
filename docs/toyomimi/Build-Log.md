# Toyomimi — Build Log

> 只追加，新的在上。一条一行：`日期 · 执行者 · 做了什么 · 结果`。

- 2026-09-22 · claude-code · App 内 Qwen 接入改为 qwen3.8 + 北京地域（`qwen_realtime.rs`：新协议、delta 累积成快照、源语言可自动识别、可选 region / workspace）；实测只需 Key、无需业务空间 ID；界面文案去掉「新加坡 / 免费预览」；重新构建 Toyomimi Dev.app · 完成，待维护者手动测试

- 2026-09-22 · claude-code · 新增 `scripts/phase0/probe.py`（音频按真实语速推流、逐事件打时间戳）；用测试视频前 60 秒验证连接：**Qwen3.8 北京地域 ✅**（译文逐字流式，对照 whisper 原文时间轴约落后 1–2.5 秒；60 秒用量 input 481 / output 293 tokens）；**Soniox Key 有效但余额为 0（402）** · Qwen 通、Soniox 待充值

- 2026-09-22 · claude-code · 装 Rust 1.98.1；上游原版编译通过；改身份为 Toyomimi（identifier / 版本 0.10.0 / 更新端点 + 自有 minisign 公钥 / 数据目录 / About 链接），去掉上游 Developer ID；新增 `ci.yml`（macOS + Windows 双平台 cargo check）；启用 fork 的 Actions；`Toyomimi Dev.app` 本机构建并启动成功 · 完成，未测真实翻译（缺 Key）

- 2026-09-22 · claude-code · fork `phuc-nt/my-translator` v0.9.1（`3495f99`，138 个上游提交保留）；`.gitignore` 补 Vibe Coding 基线并放行 `CLAUDE.md`；新增 `CLAUDE.md`、`docs/toyomimi/`（context / PLAN / Build-Log / bugs）、README 顶部 Toyomimi 区块 · 完成，代码未改动
