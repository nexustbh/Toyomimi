# Phase 0 · 引擎实测

目标：用数据回答「Qwen3.8-LiveTranslate vs Soniox 在我们的场景里谁更合适」。方法与维度见 `docs/toyomimi/PLAN.md`。

- Key 从仓库根目录 `.env` 读取（模板见 `.env.example`），**不要写进脚本**
- 测试音频放 `local/test-audio/`（gitignore），公开测试集片段可放 `scripts/phase0/fixtures/`（需确认许可）
- 原始事件日志输出到 `out/phase0/`（gitignore），结论写进 `docs/toyomimi/phase0-report.md`

- `probe.py`：连接验证 + 延迟 / 译文 / 用量初测（见文件头用法）
