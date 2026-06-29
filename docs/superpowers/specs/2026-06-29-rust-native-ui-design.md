# lan-chat — Rust 原生 UI 重写设计简报

> 这是给后续 agent 接手做实施的设计简报。所有约束和决策已和用户确认。
> 编写时间：2026-06-29
> 状态：已确认，待实施

---

## 1. 项目背景

**lan-chat** 是一个纯局域网的 P2P 桌面聊天应用。

当前架构：
- 后端（已经是纯 Rust）：`src-tauri/src/` 下的 axum WebSocket server、mdns-sd 自动发现、peer 池、消息存储
- 壳：Tauri 2 — 开窗口、把 `frontend/index.html` 塞进 WebView、暴露 `window.__TAURI__` IPC bridge
- 前端：`frontend/index.html` 一个 1500+ 行的 HTML+CSS+JS 巨文件

**功能已全部正常**，但用户认为：
- 前端视觉「不整齐、错位多、没有合理的代码布局方式」
- 整套方案「从里到外透着 NodeJS 味」，明确表达厌恶

## 2. 目标

**彻底移除 Tauri。** 用纯 Rust GUI 重写整个 UI，编译出 native binary。

后端模块（`server.rs` / `mdns.rs` / `peers.rs` / `messages.rs` / `network.rs`）**原样复用**，只去掉 Tauri 入口和 IPC bridge 层。

## 3. 锁定的决策（已与用户确认，不可改）

| 项 | 决策 |
|---|---|
| GUI 框架 | **egui** (eframe + wgpu) |
| 平台 | macOS + Ubuntu 核心，Windows 「能有则有」 |
| 视觉方向 | **A. 终端感精炼版** — amber-on-black + JetBrains Mono，保留识别度但布局锐利克制 |
| 布局 | 严格 4px/8px 网格对齐、统一 spacing 阶梯、单一字体贯穿 UI |
| 字号 | 比原 13px 整体放大（建议 baseline 14-15px，标题 18-22px） |
| 颜色 | amber 为主 + 一种点缀色（绿/灰），其余深色克制 |
| GUI 框架选择权 | 授权给 agent（用户说「不是恨清楚，自由度给你」） |
| 文件传输方案 | HTTP + 内容寻址（sha256 id）— 见 §6 |

## 4. 不可砍的功能（全部要保留）

1. 实时聊天（axum WebSocket 广播）
2. 一键剪贴板共享（弹窗 + 预填系统剪贴板）
3. mDNS 自动发现（mdns-sd）
4. 200 条内存消息历史（重启清空）
5. 500 条 ID 滑动窗口去重防回环
6. 手动 IP:port 连接（兜底 mDNS 不通的场景）
7. 状态栏（peer 数 / 本机 IP / 改名 / 手动连接）
8. **新增**：文件传输（大小都要支持）

## 5. 架构

```
src-tauri/src/
├── main.rs              # 启动入口（不再需要 Tauri::Builder）
├── lib.rs               # 装配：start_server + start_mdns + app::run
├── app.rs               # eframe::App 实现，egui 整棵 UI 树在这里
├── server.rs            # （原样）axum WS server
├── mdns.rs              # （原样）mDNS 注册+浏览
├── peers.rs             # （原样）PeerPool
├── messages.rs          # （原样）MessageStore（200 条环形）
├── network.rs           # （原样）网络接口
├── transfer.rs          # 新增：HTTP 文件上传/下载 + sha256 校验
└── ui/                  # 新增：egui widgets 分模块
    ├── mod.rs
    ├── setup.rs         # 启动 setup overlay（命名终端）
    ├── log_view.rs      # 消息流渲染
    ├── input_bar.rs     # 输入栏
    ├── status_bar.rs    # 状态栏（peer/IP/改名/手动连接）
    ├── clip_modal.rs    # 剪贴板分享弹窗
    ├── file_card.rs     # 文件消息卡片
    └── theme.rs         # 颜色/字体/间距常量（8px 网格）
```

**线程模型**：
- `tokio::runtime::Runtime` — 跑 axum server / mDNS / 文件传输 async 任务
- eframe 主线程 — 跑 `egui` 渲染
- 跨线程通信：`tokio::sync::mpsc` channel（GUI ← 后端事件）、`Arc<Mutex<>>` 状态共享

**文件传输 HTTP 端点**（在原 axum 路由上加）：
- `POST /upload` — 接收方暴露给发送方（multipart 或 raw body），body 是文件原始字节
- `GET /files/:sha256` — 下载

## 6. 文件传输详细方案（已确认）

**核心思想：HTTP 传 body，WebSocket 传控制。**

- **WebSocket 通道**（只做控制面）：
  - 聊天文本消息
  - 剪贴板消息
  - **文件元数据消息**（新增类型 `"file"`，载荷 `{id, sha256, filename, size, download_url, sender, ts}`）
  - 走现有协议，500 条 ID 滑动窗口去重窗口自动覆盖（`file.id` 也进窗口）

- **HTTP 通道**（只做 body）：
  - 发送方：UI 拖入/选文件 → Rust 算 sha256 + size → 选一个 peer 的上传端点 `POST /upload` → 推送 raw bytes
  - 接收方：用户点聊天里的文件卡片 → Rust 后台 `GET /files/:sha256` → 弹原生保存对话框 → 落盘
  - 不做分块 / 断点续传 / 进度条（LAN 环境下用不上；后续可加）

- **内容寻址**：文件 id = sha256，重复文件天然 dedupe

- **接收方体验**：
  - 在聊天流里以**文件卡片**形式展示（带文件名、size、sha256 短码、下载按钮）
  - 接收方点下载 → 弹原生 save dialog → 落盘
  - 不自动保存（用户隐私）

- **发送方体验**：
  - 拖文件到输入栏区域 / 或点 + 按钮选文件
  - 算完 sha256 后，立刻在聊天流里以「我发送的文件」卡片出现（带「已发送 / 失败」状态）
  - 进度：暂时不显示（LAN 够快；后续可加）

## 7. 视觉设计原则（frontend-design skill 会深化）

**精炼版终端感** — 保留 amber caret / JetBrains Mono / 矩形卡片这些核心识别度，但解决现在「不整齐、错位多」的问题：

- **8px 网格**：所有 `padding` / `margin` / `gap` 是 4 的倍数（0/4/8/12/16/24/32）
- **统一 spacing scale**：`space_xs/sm/md/lg/xl` 五个等级
- **字号阶梯**：`text_xs(12) / text_sm(13) / text_base(15) / text_lg(18) / text_xl(22)` — base 比原 13 大
- **行高**：1.5（标题）/ 1.6（正文）
- **字体**：JetBrains Mono 唯一字体，连 UI 标签、按钮、状态文本都用等宽
- **颜色**：
  - `bg` #0a0a0a
  - `bg_elev` #111211
  - `line` #1f201d
  - `amber` #ffb000（主色）
  - `green` #7cffb2（点缀：在线/成功）
  - `text` #d9d6cc
  - `muted` #5c5d54
- **不沿用** 现在的 inline style 杂糅（HTML 里直接写 `style="..."` 那种）— 所有视觉属性进 `theme.rs` 常量，组件只引用变量名

**具体 UI 布局**（与现在功能等价但布局更清晰）：
- 顶部 40px header：品牌 + online 状态
- 中间 flex-1 log：消息流（chat 行 / file 卡片 / clipboard 行）
- 底部 56px input bar：输入框 + 字数 + 发送 + 拖文件 + clip 按钮
- 底部 32px status bar：peer 数 + 本机 IP + 改名 + 手动 IP 连接

## 8. 数据流

```
peer 端
  → WebSocket 文本消息
  → axum server::handle_connection
  → 解析 JSON，匹配 type (hello/text/clipboard/file)
  → 更新 PeerPool / MessageStore
  → 走 mpsc channel 推 GUI
  → egui::App::update 消费事件
  → 调用 ui/log_view.rs::push_message(msg)
  → render

用户输入
  → egui TextEdit (keydown Enter)
  → async tokio task: invoke "send_message" 等价物（直接调 axum broadcast）
  → 走 mpsc 回流（自己发的消息也要在 log 里显示）
```

## 9. 错误处理

- 启动时：mDNS 失败不阻塞主程序，仅 log warn
- WebSocket 断线：自动重连（指数退避），UI 显示「reconnecting」状态
- 文件发送失败：消息卡片显示「failed」状态，可重试
- 文件接收失败：弹 toast，下载按钮变红
- 剪贴板读不到权限：弹窗 textarea 为空，用户手填

## 10. 测试

- 单元测试：`peers.rs` / `messages.rs` / `transfer.rs` 的纯逻辑
- 集成测试：起一个 axum server，模拟两个 peer 互相发消息和文件
- 手动验证：macOS + Ubuntu 各跑一遍，所有功能（chat/clip/mDNS/file/手动连接/改名）走通
- 文件传输：至少测一次 1KB / 1MB / 100MB 三种大小

## 11. 实施顺序（建议）

1. **新建 `app.rs` + `ui/theme.rs` + `ui/setup.rs`** — 最小的 eframe 窗口能起来，能跑 setup overlay
2. **复用后端** — 改 `main.rs` 启动 axum + mDNS，去掉 Tauri
3. **实现 log_view / input_bar** — 聊天能发能收
4. **实现 status_bar** — peer/IP/改名/手动连接
5. **实现 clip_modal** — 剪贴板分享
6. **新增 `transfer.rs` + HTTP 端点** — 文件传输
7. **实现 file_card** — 文件消息渲染 + 下载
8. **frontend-design skill** — 在所有功能跑通后，重新设计视觉，输出 mockup，应用到 `theme.rs`
9. **打磨** — 动画、错误状态、空状态、键盘快捷键

## 12. 依赖变更（src-tauri/Cargo.toml）

**移除**：
- `tauri`
- `tauri-build`（build-dependencies）

**新增**：
- `eframe = "0.27"`（egui 0.27 + wgpu）
- `egui = "0.27"`
- `arboard = "3"`（系统剪贴板，egui 自带也能用，但 arboard 更稳）
- `rfd = "0.14"`（原生 save dialog）
- `sha2 = "0.10"`

**保留**：
- axum / tokio / mdns-sd / serde / uuid / if-addrs / dashmap / anyhow / tokio-tungstenite / tracing

## 13. 完成定义

- [ ] `cargo run` 直接出窗口，无 Tauri、无 WebView、无 NodeJS、无 JS 文件
- [ ] 所有 8 项功能（含文件传输）走通
- [ ] macOS + Ubuntu 各验证一次
- [ ] 所有 inline style 从 HTML/JS 中清除（这部分已被 Rust 重写自然解决）
- [ ] 字号比原版大、间距统一 4/8 倍数
