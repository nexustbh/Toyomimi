# Toyomimi — 踩坑记录

> 编号连续，一坑一条：症状 · 原因 · 解法。

## B1 · 上游身份信息未改会导致「被上游自动更新」（预防性）

- **症状**：我们打包的 App 在设置里检查更新，会下载并安装 my-translator 官方版本，覆盖我们的改动。
- **原因**：`src-tauri/tauri.conf.json` 的 updater `endpoints` 指向 `phuc-nt/my-translator` releases，`pubkey` 也是上游的。
- **解法**：第 1 阶段首个任务——改 `productName` / `identifier` / updater `endpoints` + 生成我们自己的签名密钥对。在此之前不要分发任何构建。
