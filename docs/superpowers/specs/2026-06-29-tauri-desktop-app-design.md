# LAN Chat — Tauri Desktop App 设计规格

**日期：** 2026-06-29  
**状态：** 已审核通过

---

## 目标

将现有 Node.js LAN Chat 应用完整迁移为 Tauri + Rust 桌面应用，消除 Node.js 依赖。每台设备运行一个对等节点（P2P），通过 mDNS 自动发现 + 手动 IP 兜底连接局域网内其他设备，复用现有 HTML/CSS 前端。支持 Ubuntu 和 macOS。

---

## 架构概述

每个 Tauri 实例同时扮演"服务器"和"客户端"：
- 在本机启动一个 Axum WebSocket 服务器（监听随机或固定端口）
- 通过 mDNS 广播自身存在，并监听其他节点广播
- 与发现的每个对等节点建立 WebSocket 连接（连接池）
- 发消息时向所有已连接 peer 广播
- Tauri WebView 加载本地 HTML 前端，通过 Tauri IPC command 与 Rust 层通信

前端通信层从 Socket.IO 改为 Tauri IPC（`invoke` + `listen`），其余 HTML/CSS 几乎不变。

---

## 网卡过滤策略

目标：在复杂网络环境（VPN tun、雷电网桥、多网卡、代理）下，准确识别适合局域网通信的网卡。

### 自动排除（硬过滤）
- `lo` / loopback（`127.x.x.x`）
- VPN/tunnel 接口：名称匹配 `tun*`, `utun*`, `wg*`, `ppp*`, `ipsec*`, `tailscale*`
- Docker/虚拟网桥：`docker0`, `br-*`, `virbr*`, `veth*`, `vmnet*`
- IPv6-only 地址（仅保留 IPv4）
- 链路本地地址（`169.254.x.x`）

### 保留候选（软过滤，用户可调整）
- 无线：`wlan*`, `wlp*`, `en0`（macOS 无线）
- 有线以太网：`eth*`, `enp*`, `en1`, `en2`（macOS 有线）
- 雷电网桥：`bridge*`, `thunderbolt*`, `en3`+ 等 macOS 雷电接口
- 所有其他未命中硬过滤规则的私有 IP 段地址（`10.x`, `192.168.x`, `172.16-31.x`）

### 用户界面
设置面板显示所有候选网卡（接口名 + IP），用户可勾选/取消。选定的网卡用于：
1. mDNS 广播绑定接口
2. WebSocket 服务器监听地址
3. 在"本机 IP"面板展示给用户，供手动分享

---

## P2P 连接模型

### 节点身份
每个节点启动时生成一个 UUID（存 app data 目录），作为唯一 ID，避免自连。

### mDNS
- 服务类型：`_lanchat._tcp.local.`
- TXT 记录：`node_id=<uuid>`, `nickname=<用户昵称>`
- 在每个选定网卡上独立注册服务
- 发现到新节点后，检查 `node_id` 去重，过滤自身，发起 WebSocket 连接

### WebSocket 连接
- 每个节点监听端口默认 `4242`，端口冲突时自动递增尝试（最多 +10）
- 连接握手：双方交换 `{node_id, nickname, version}` JSON
- 断线自动重连，指数退避（1s → 2s → 4s → max 30s）
- 节点列表（`peers.rs`）维护所有活跃连接

### 消息广播
- 发送方向所有 peer 发送消息
- 消息带 `id`（`timestamp_random`），接收方去重（防止 A→B→A 环路）
- 消息格式与现有保持一致（`id, text, device, type, ts`）

---

## 消息存储

内存环形缓冲，最多 200 条，应用退出即清空（同现有行为）。新 peer 连接后，向其发送本地 history（最近 200 条）。

---

## 前端改造

现有 `public/index.html` 的改造点：
1. 移除 `<script src="/socket.io/socket.io.js">` 和所有 `io()` 调用
2. 新增 Tauri IPC 调用层（`window.__TAURI__.core.invoke`）
3. 事件监听改为 `window.__TAURI__.event.listen`

| 原 Socket.IO | 新 Tauri IPC |
|---|---|
| `socket.emit('message', data)` | `invoke('send_message', {text, type})` |
| `socket.on('message', cb)` | `listen('message', cb)` |
| `socket.on('history', cb)` | `invoke('get_history')` → 返回数组 |
| `socket.on('peer_list', cb)` | `listen('peer_update', cb)` |

新增 UI 元素：
- 设置面板：网卡选择、昵称设置
- 状态栏：显示已连接 peer 数量、本机 IP 列表
- 手动添加 peer：输入框 + "连接" 按钮（`invoke('connect_peer', {ip, port})`）

---

## 文件结构

```
lan-chat/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Tauri 构建入口，注册 commands，启动后台任务
│   │   ├── server.rs        # Axum WebSocket 服务器（接受入站连接）
│   │   ├── peers.rs         # Peer 连接池：管理所有出站+入站 WS 连接
│   │   ├── mdns.rs          # mDNS 广播 + 发现（mdns-sd crate）
│   │   ├── network.rs       # 网卡枚举 + 智能过滤
│   │   ├── messages.rs      # 消息结构体、环形缓冲、去重 ID 集合
│   │   └── commands.rs      # Tauri IPC commands（前端调用入口）
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/               # 应用图标
├── src/
│   └── index.html           # 改造后的前端（移除 Socket.IO，接入 Tauri IPC）
├── package.json             # Tauri CLI 构建脚本
└── docs/
    └── superpowers/
```

### Rust 依赖（Cargo.toml）

```toml
[dependencies]
tauri          = { version = "2", features = ["protocol-asset"] }
axum           = { version = "0.7", features = ["ws"] }
tokio          = { version = "1", features = ["full"] }
tokio-tungstenite = "0.23"
mdns-sd        = "0.10"
serde          = { version = "1", features = ["derive"] }
serde_json     = "1"
uuid           = { version = "1", features = ["v4"] }
if-addrs       = "0.13"          # 网卡枚举，跨平台
dashmap        = "6"             # 并发 peer 连接 map
futures        = "0.3"
anyhow         = "1"
```

---

## 跨平台构建目标

| 平台 | WebView | 构建方式 |
|------|---------|---------|
| Ubuntu 22.04+ | WebKitGTK 4.1 | `cargo tauri build` |
| macOS 12+ | WKWebView | `cargo tauri build` |

构建产物：
- Ubuntu：`.deb` 包 + AppImage
- macOS：`.dmg` + `.app`

---

## 不在本次范围内

- Windows 支持
- HTTPS / TLS 加密（局域网内部，暂不实现）
- 消息持久化
- 文件传输
- 端对端加密
