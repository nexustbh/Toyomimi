"""Phase 0 probe: stream an audio file at real-time speed to Qwen3.8-LiveTranslate
and/or Soniox, log every server event with a timestamp, print a summary.

Usage (from repo root):
    uv run --with websockets scripts/phase0/probe.py local/test-audio/x.mp4 \
        --engines qwen,soniox --seconds 60 --target zh

Keys come from .env (never printed). Raw event logs -> out/phase0/<run>/<engine>.jsonl
"""

from __future__ import annotations

import argparse
import asyncio
import base64
import json
import os
import subprocess
import time
import uuid
from pathlib import Path

import websockets

ROOT = Path(__file__).resolve().parents[2]
RATE = 16000
CHUNK_MS = 100
CHUNK_BYTES = RATE * 2 * CHUNK_MS // 1000  # 16-bit mono

QWEN_MODEL = "qwen3.8-livetranslate-flash-realtime"
QWEN_HOSTS = {
    "cn-beijing": "dashscope.aliyuncs.com",
    "ap-southeast-1": "dashscope-intl.aliyuncs.com",
}
SONIOX_URL = "wss://stt-rt.soniox.com/transcribe-websocket"
SONIOX_MODEL = "stt-rt-v5"


def load_env() -> dict[str, str]:
    env = {}
    for line in (ROOT / ".env").read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            env[k.strip()] = v.strip().strip("\"'")
    return env


def decode_pcm(path: str, seconds: float, offset: float) -> bytes:
    cmd = ["ffmpeg", "-v", "error", "-ss", str(offset), "-i", path, "-t", str(seconds),
           "-ac", "1", "-ar", str(RATE), "-f", "s16le", "-"]
    return subprocess.run(cmd, check=True, capture_output=True).stdout


class Log:
    def __init__(self, path: Path):
        self.f = path.open("w")
        self.t0 = None

    def start(self):
        self.t0 = time.monotonic()

    def now(self) -> float:
        return time.monotonic() - self.t0 if self.t0 else 0.0

    def write(self, direction: str, obj):
        self.f.write(json.dumps({"t": round(self.now(), 3), "dir": direction, "ev": obj},
                                ensure_ascii=False) + "\n")
        self.f.flush()


async def pump(pcm: bytes, send_chunk, log: Log, audio_done: dict):
    """Send PCM in CHUNK_MS pieces paced to wall-clock (real-time)."""
    log.start()
    n = 0
    for i in range(0, len(pcm), CHUNK_BYTES):
        await send_chunk(pcm[i:i + CHUNK_BYTES])
        n += 1
        target = n * CHUNK_MS / 1000
        delay = target - log.now()
        if delay > 0:
            await asyncio.sleep(delay)
    audio_done["t"] = log.now()


# ─────────────────────────── Qwen3.8 ───────────────────────────

async def run_qwen(env, pcm, target, outdir: Path) -> dict:
    region = env.get("DASHSCOPE_REGION", "cn-beijing")
    url = f"wss://{QWEN_HOSTS[region]}/api-ws/v1/realtime?model={QWEN_MODEL}"
    headers = {"Authorization": f"Bearer {env['DASHSCOPE_API_KEY']}"}
    if env.get("DASHSCOPE_WORKSPACE_ID"):
        headers["X-DashScope-WorkSpace"] = env["DASHSCOPE_WORKSPACE_ID"]
    log = Log(outdir / "qwen.jsonl")
    res = {"engine": "qwen", "url": url, "src": [], "tr": [], "errors": [], "usage": []}
    ready = asyncio.Event()
    audio_done: dict = {}

    async with websockets.connect(url, additional_headers=headers, max_size=None) as ws:
        res["connected"] = True

        async def reader():
            async for msg in ws:
                ev = json.loads(msg)
                log.write("in", ev)
                t = ev.get("type", "")
                if t == "session.created":
                    await ws.send(json.dumps({"type": "session.update", "session": {
                        "output_modalities": ["text"],
                        "translation": {"language": target},
                        "audio": {"output": {"voice": "Tina"}},
                    }}))
                elif t == "session.updated":
                    ready.set()
                elif t == "conversation.item.input_audio_transcription.completed":
                    res["src"].append((log.now(), ev.get("transcript", ""),
                                       ev.get("speaker") or ev.get("speaker_id")))
                elif t == "conversation.item.input_audio_transcription.delta":
                    res.setdefault("first_src_delta", log.now())
                elif t == "response.text.delta":
                    res.setdefault("first_tr_delta", log.now())
                elif t == "response.text.done":
                    res["tr"].append((log.now(), ev.get("text", "")))
                elif t == "response.done":
                    u = ev.get("response", {}).get("usage")
                    if u:
                        res["usage"].append(u)
                elif t == "error":
                    res["errors"].append(ev.get("error", ev))
                    ready.set()
                elif t == "session.finished":
                    return

        rtask = asyncio.create_task(reader())
        try:
            await asyncio.wait_for(ready.wait(), 15)
        except asyncio.TimeoutError:
            res["errors"].append("session.updated timeout")
        if res["errors"]:
            rtask.cancel()
            return res
        res["session_ok"] = True

        async def send_chunk(b):
            await ws.send(json.dumps({"event_id": "event_" + uuid.uuid4().hex[:20],
                                      "type": "input_audio_buffer.append",
                                      "audio": base64.b64encode(b).decode()}))

        await pump(pcm, send_chunk, log, audio_done)
        # trailing silence lets server VAD close the last segment
        for _ in range(20):
            await send_chunk(b"\x00" * CHUNK_BYTES)
            await asyncio.sleep(CHUNK_MS / 1000)
        await ws.send(json.dumps({"type": "session.finish"}))
        try:
            await asyncio.wait_for(rtask, 15)
        except asyncio.TimeoutError:
            rtask.cancel()
    res["audio_end"] = audio_done.get("t")
    return res


# ─────────────────────────── Soniox ───────────────────────────

async def run_soniox(env, pcm, target, outdir: Path) -> dict:
    log = Log(outdir / "soniox.jsonl")
    res = {"engine": "soniox", "url": SONIOX_URL, "src": [], "tr": [], "errors": [], "usage": []}
    audio_done: dict = {}
    cur_src, cur_tr = [], []

    async with websockets.connect(SONIOX_URL, max_size=None) as ws:
        res["connected"] = True
        await ws.send(json.dumps({
            "api_key": env["SONIOX_API_KEY"], "model": SONIOX_MODEL,
            "audio_format": "pcm_s16le", "sample_rate": RATE, "num_channels": 1,
            "enable_endpoint_detection": True, "enable_speaker_diarization": True,
            "enable_language_identification": True,
            "translation": {"type": "one_way", "target_language": target},
        }))
        res["session_ok"] = True

        async def reader():
            async for msg in ws:
                ev = json.loads(msg)
                log.write("in", ev)
                if ev.get("error_code") or ev.get("error_message"):
                    res["errors"].append({"code": ev.get("error_code"), "message": ev.get("error_message")})
                for tok in ev.get("tokens", []):
                    st = tok.get("translation_status")
                    if st == "translation":
                        res.setdefault("first_tr_delta", log.now())
                    elif st in ("original", "none"):
                        res.setdefault("first_src_delta", log.now())
                    if not tok.get("is_final"):
                        continue
                    if tok.get("text") == "<end>":
                        if cur_src:
                            res["src"].append((log.now(), "".join(cur_src).strip(), None))
                            cur_src.clear()
                        if cur_tr:
                            res["tr"].append((log.now(), "".join(cur_tr).strip()))
                            cur_tr.clear()
                        continue
                    (cur_tr if st == "translation" else cur_src).append(tok.get("text", ""))
                if ev.get("finished"):
                    res["usage"].append({k: ev.get(k) for k in ("total_audio_proc_ms", "final_audio_proc_ms") if k in ev})
                    return

        rtask = asyncio.create_task(reader())

        async def send_chunk(b):
            await ws.send(b)

        await pump(pcm, send_chunk, log, audio_done)
        await ws.send(b"")  # end of stream
        try:
            await asyncio.wait_for(rtask, 20)
        except asyncio.TimeoutError:
            rtask.cancel()
        if cur_src:
            res["src"].append((log.now(), "".join(cur_src).strip(), None))
        if cur_tr:
            res["tr"].append((log.now(), "".join(cur_tr).strip()))
    res["audio_end"] = audio_done.get("t")
    return res


# ─────────────────────────── main ───────────────────────────

def summarize(r: dict):
    print(f"\n══ {r['engine']} ══  {r['url']}")
    print(f"  连接: {'✅' if r.get('connected') else '❌'}   会话: {'✅' if r.get('session_ok') else '❌'}")
    for e in r["errors"]:
        print(f"  ❌ 错误: {e}")
    if "first_src_delta" in r:
        print(f"  首个原文返回: {r['first_src_delta']:.2f}s（从开始推流算）")
    if "first_tr_delta" in r:
        print(f"  首个译文返回: {r['first_tr_delta']:.2f}s")
    print(f"  原文段数: {len(r['src'])}   译文段数: {len(r['tr'])}（按停顿分段；字幕是逐字实时推送的）")
    speakers = {s for _, _, s in r["src"] if s}
    if speakers:
        print(f"  说话人标签: {sorted(speakers)}")
    for t, txt in r["tr"][:5]:
        print(f"    [{t:6.2f}s] {txt[:80]}")
    if r["usage"]:
        print(f"  用量: {json.dumps(r['usage'][-1], ensure_ascii=False)[:300]}")


async def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("audio")
    ap.add_argument("--engines", default="qwen,soniox")
    ap.add_argument("--seconds", type=float, default=60)
    ap.add_argument("--offset", type=float, default=0)
    ap.add_argument("--target", default="zh")
    a = ap.parse_args()

    env = load_env()
    pcm = decode_pcm(a.audio, a.seconds, a.offset)
    outdir = ROOT / "out" / "phase0" / time.strftime("%Y%m%d-%H%M%S")
    outdir.mkdir(parents=True, exist_ok=True)
    print(f"音频 {len(pcm) / RATE / 2:.1f}s，按真实语速推流 → {a.engines}，目标语言 {a.target}")

    runners = {"qwen": run_qwen, "soniox": run_soniox}
    async def safe(e):
        try:
            return await runners[e](env, pcm, a.target, outdir)
        except websockets.ConnectionClosed as exc:
            # server closed early — the reason is usually in the logged events
            path = outdir / f"{e}.jsonl"
            errs = [json.loads(l)["ev"] for l in path.read_text().splitlines()
                    if "error" in l] if path.exists() else []
            return {"engine": e, "url": "", "connected": True, "src": [], "tr": [], "usage": [],
                    "errors": [{k: v for k, v in ev.items() if k.startswith("error")} for ev in errs]
                    or [f"connection closed: {exc}"]}

    tasks = [safe(e) for e in a.engines.split(",")]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    for e, r in zip(a.engines.split(","), results):
        if isinstance(r, Exception):
            print(f"\n══ {e} ══\n  ❌ {type(r).__name__}: {r}")
        else:
            summarize(r)
            (outdir / f"{e}.summary.json").write_text(json.dumps(r, ensure_ascii=False, indent=1))
    print(f"\n原始事件日志: {outdir.relative_to(ROOT)}/")


if __name__ == "__main__":
    asyncio.run(main())
