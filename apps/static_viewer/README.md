# OE1022D Dataset Static Viewer

> C10 第一个可达的版本。**不依赖 Tauri、不依赖 node_modules**——
> 纯 Python http server + 单 HTML 文件 + Plotly.js (CDN)。

## 用法

```sh
cd /Users/erictseng/Documents/codex_git/odmr-dataset-acquisition
python3 apps/static_viewer/serve.py --port 8000
# Open http://127.0.0.1:8000/ in a browser
```

Server 列出 `runs/<run_id>/samples.ndjson` 文件,你点哪个就 Plotly 渲染哪个。

## 架构

```
Browser  ──GET /──  index.html (Plotly.js via CDN)
       ──GET /api/runs──  JSON 列出 runs/
       ──GET /api/runs/<id>/samples.ndjson──  14 MB NDJSON 流
       ──JS 解析 + Plotly.react() 渲染
```

## 控件

- **Run 文件列表**:点哪个就跑哪个的图
- **Play / Pause / Reset**:50 Hz 播放速率,把真机 5 min 数据按 50x 加速回放 ≈ 6 秒看完整段
- **Speed 滑块**:0.5x ~ 200x 回放速率
- **Visible window 滑块**:100 ~ 5000 sample 滑窗,超过 reset(单方向 X 轴,符合 PRD §7.2)
- **3 个 trace**:B-X (V) 左轴,B-Y (V) 左轴,B-Freq (Hz) 右轴(双 Y 轴)

## 为什么是 Python + Plotly.js,不是 Tauri + React?

按 v0.1 的"先让你看到图"原则:
- Tauri v2 + React + pnpm 装依赖 + 配 Tauri 壳子要 30+ 分钟
- 静态 server + CDN 加载 Plotly 是 5 分钟
- 真正的实时流(30 Hz live emit)需要 Rust SSE/WS server,**留到 v0.2**
- v0.1 验证"后端采集 → ndjson 落盘 → 前端渲染"整条链路**已通**

数据契约是同一个(1 ndjson line = 1 sample),所以 v0.2 只需要把"读 ndjson"换成
"接 SSE/WS live",前端代码 0 改动。

## 当前数据(2026-06-05_real_5min)

- **331 frames / 49650 samples / 5 min 00.48s**
- 真机 D6130220, USB CDC 921600 baud
- 1 warmup 帧(13260 bytes, 截断到 12288, 标记 partial_warmup=true)
- 0 丢失帧
- 见 `runs/2026-06-05_real_5min/REPORT.md`
