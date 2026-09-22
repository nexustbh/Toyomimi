# Toyomimi — 踩坑记录

> 编号连续，一坑一条：症状 · 原因 · 解法。

## B1 · 上游身份信息未改会导致「被上游自动更新」（预防性）· ✅ 2026-09-22 已修

- **症状**：我们打包的 App 在设置里检查更新，会下载并安装 my-translator 官方版本，覆盖我们的改动。
- **原因**：`src-tauri/tauri.conf.json` 的 updater `endpoints` 指向 `phuc-nt/my-translator` releases，`pubkey` 也是上游的。
- **解法**：第 1 阶段首个任务——改 `productName` / `identifier` / updater `endpoints` + 生成我们自己的签名密钥对。在此之前不要分发任何构建。

## B2 · ad-hoc 签名的开发版，每次重建都要重新授予录屏权限

- **症状**：重新 `npm run build:dev` 之后，App 采集不到系统声音 / 又弹录屏授权。
- **原因**：macOS 的隐私权限（TCC）绑定代码签名；ad-hoc 签名（`-`）每次构建都不同。
- **解法**：短期——重建后到「系统设置 → 隐私与安全性 → 录屏与系统录音」里把旧的 Toyomimi Dev 删掉再重新勾上。长期——在钥匙串建一个稳定的自签名证书，写进 `.env` 的 `APP_SIGNING_IDENTITY`（涉及钥匙串，需维护者确认后再做）。

## B3 · `gh` 命令默认指向上游仓库

- **症状**：`gh workflow run` 报 404，URL 里是 `phuc-nt/my-translator`。
- **原因**：仓库有 `upstream` 远端，gh 会优先选它。
- **解法**：已执行 `gh repo set-default nexustbh/Toyomimi`（写在本地 `.git/config`，新克隆需重做）。

## B4 · 用 `open` 启动后首次看不到窗口

- **症状**：2026-09-22 首次 `open Toyomimi Dev.app`，进程在但没有窗口；杀掉后直接运行二进制，窗口正常出现。
- **原因**：未确认（可能是首次启动的初始化或窗口出现在别的桌面空间）。复现时再查。

## B5 · 上游的 Qwen 接入是旧模型、国际站，协议与 3.8 不兼容

- **症状**：用北京地域的 Key 连不上 / 3.8 模型报错。
- **原因**：`src-tauri/src/commands/qwen_realtime.rs` 写死 `dashscope-intl`（新加坡）+ `qwen3-livetranslate-flash-realtime`（旧模型）。3.8 的协议变了：`session.update` 用 `output_modalities`（不是 `modalities`），必须带 `audio.output.voice`（否则默认音色报 400）；原文走 `conversation.item.input_audio_transcription.delta/.completed`，译文走 `response.text.delta/.done`（旧版是 `response.text.text`）；源语言可以省略（自动识别）。
- **解法**：第 1 阶段重写 Qwen 接入：地域可选（北京 `dashscope.aliyuncs.com` / 新加坡 `dashscope-intl.aliyuncs.com`），协议按 `scripts/phase0/probe.py` 已验证的版本。参考实现：`huankechong/QwenLiveTranslate`（MIT）的 `realtime_client.py`。
