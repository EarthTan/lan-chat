# LAN Chat Tauri Desktop App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 LAN Chat 从 Node.js 完整迁移为 Tauri 2 + Rust 跨平台桌面应用，P2P 架构，mDNS 自动发现 + 手动 IP 兜底，支持 Ubuntu 和 macOS。

**Architecture:** 每个 Tauri 实例在本机启动 Axum WebSocket 服务器，通过 mdns-sd 在选定网卡上广播自身并发现 peer，与所有 peer 维持 WebSocket 长连接组成全连接网格。前端复用现有 HTML/CSS，通信层从 Socket.IO 替换为 Tauri IPC（invoke + listen）。

**Tech Stack:** Tauri 2, Rust (axum 0.7, tokio 1, mdns-sd 0.10, tokio-tungstenite 0.23, if-addrs 0.13, dashmap 6, serde_json 1, uuid 1)

---

## 前置检查

- [ ] **确认系统依赖已安装**

  **Ubuntu：**
  ```bash
  sudo apt update && sudo apt install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev patchelf curl build-essential pkg-config
  ```

  **macOS：**
  ```bash
  xcode-select --install
  ```

  **两者都需要：**
  ```bash
  # 安装 Rust（若未安装）
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustup update stable

  # 安装 Tauri CLI
  cargo install tauri-cli --version "^2"
  ```

  验证：
  ```bash
  cargo tauri --version
  # 期望输出：tauri-cli 2.x.x
  ```

- [ ] **确认项目根目录**

  所有命令在 `lan-chat/` 目录下执行（即含 `server.js` 的目录）。

---

## Task 1: 初始化 Tauri 2 项目结构

**Files:**
- Create: `lan-chat/src-tauri/Cargo.toml`
- Create: `lan-chat/src-tauri/tauri.conf.json`
- Create: `lan-chat/src-tauri/build.rs`
- Create: `lan-chat/src-tauri/src/main.rs`（骨架）
- Create: `lan-chat/package.json`（更新）
- Create: `lan-chat/src/index.html`（占位，后续 Task 覆盖）

- [ ] **Step 1: 在 lan-chat/ 运行 Tauri 初始化**

  ```bash
  cd lan-chat
  cargo tauri init --ci \
    --app-name "LAN Chat" \
    --window-title "LAN Chat" \
    --dist-dir "../src" \
    --dev-url "" \
    --before-dev-command "" \
    --before-build-command ""
  ```

  期望：生成 `src-tauri/` 目录，内含 `Cargo.toml`、`tauri.conf.json`、`src/main.rs`、`build.rs`。

- [ ] **Step 2: 替换 src-tauri/Cargo.toml 为正确依赖**

  覆盖 `lan-chat/src-tauri/Cargo.toml`：

  ```toml
  [package]
  name = "lan-chat"
  version = "1.0.0"
  edition = "2021"

  [lib]
  name = "lan_chat_lib"
  crate-type = ["staticlib", "cdylib", "rlib"]

  [[bin]]
  name = "lan-chat"
  path = "src/main.rs"

  [build-dependencies]
  tauri-build = { version = "2", features = [] }

  [dependencies]
  tauri        = { version = "2", features = ["protocol-asset"] }
  tauri-plugin-shell = "2"
  axum         = { version = "0.7", features = ["ws"] }
  tokio        = { version = "1", features = ["full"] }
  tokio-tungstenite = { version = "0.23", features = ["native-tls"] }
  futures      = "0.3"
  mdns-sd      = "0.10"
  serde        = { version = "1", features = ["derive"] }
  serde_json   = "1"
  uuid         = { version = "1", features = ["v4"] }
  if-addrs     = "0.13"
  dashmap      = "6"
  anyhow       = "1"
  tracing      = "0.1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  ```

- [ ] **Step 3: 更新 tauri.conf.json**

  覆盖 `lan-chat/src-tauri/tauri.conf.json`：

  ```json
  {
    "$schema": "https://schema.tauri.app/config/2",
    "productName": "LAN Chat",
    "version": "1.0.0",
    "identifier": "io.github.lan-chat",
    "build": {
      "frontendDist": "../src"
    },
    "app": {
      "windows": [
        {
          "title": "LAN Chat",
          "width": 960,
          "height": 700,
          "minWidth": 600,
          "minHeight": 480,
          "resizable": true,
          "fullscreen": false,
          "center": true
        }
      ],
      "security": {
        "csp": null
      }
    },
    "bundle": {
      "active": true,
      "targets": "all",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    }
  }
  ```

- [ ] **Step 4: 更新根 package.json**

  覆盖 `lan-chat/package.json`：

  ```json
  {
    "name": "lan-chat",
    "version": "1.0.0",
    "description": "Pure-LAN P2P chat desktop app",
    "scripts": {
      "tauri": "cargo tauri",
      "dev": "cargo tauri dev",
      "build": "cargo tauri build"
    },
    "devDependencies": {
      "@tauri-apps/cli": "^2"
    }
  }
  ```

- [ ] **Step 5: 创建 src/ 目录和占位 index.html**

  ```bash
  mkdir -p lan-chat/src
  touch lan-chat/src/index.html
  ```

- [ ] **Step 6: 生成默认图标（使用 Tauri 内置图标生成）**

  ```bash
  cd lan-chat
  # 先确保 icons 目录存在
  mkdir -p src-tauri/icons
  # 用 tauri icon 命令生成（需要一张 1024x1024 PNG，或用默认）
  cargo tauri icon --help
  ```

  若没有自定义图标，手动复制 Tauri 默认图标：
  ```bash
  # Tauri init 已自动生成 icons/，跳过此步即可
  ls lan-chat/src-tauri/icons/
  ```

- [ ] **Step 7: 验证 Cargo 依赖可拉取**

  ```bash
  cd lan-chat/src-tauri
  cargo fetch
  ```

  期望：所有依赖下载成功，无 error。

- [ ] **Step 8: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/ src/ package.json
  git commit -m "chore: initialize Tauri 2 project structure"
  ```

---

## Task 2: messages.rs — 消息结构体与环形缓冲

**Files:**
- Create: `lan-chat/src-tauri/src/messages.rs`

- [ ] **Step 1: 创建 messages.rs**

  ```rust
  // lan-chat/src-tauri/src/messages.rs
  use serde::{Deserialize, Serialize};
  use std::collections::HashSet;
  use std::sync::Mutex;

  pub const MAX_MESSAGES: usize = 200;
  /// 去重窗口：记录最近收到的消息 ID，防止 P2P 环路广播
  pub const DEDUP_WINDOW: usize = 500;

  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct Message {
      pub id: String,
      pub text: String,
      pub device: String,
      #[serde(rename = "type")]
      pub msg_type: MsgType,
      pub ts: u64,
  }

  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
  #[serde(rename_all = "lowercase")]
  pub enum MsgType {
      Text,
      Clipboard,
  }

  impl Message {
      pub fn new(text: String, device: String, msg_type: MsgType) -> Self {
          let ts = std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)
              .unwrap_or_default()
              .as_millis() as u64;
          let rand: u32 = rand_u32();
          Self {
              id: format!("{}_{:05x}", ts, rand & 0xFFFFF),
              text: text.trim().chars().take(8000).collect(),
              device: device.trim().chars().take(40).collect(),
              msg_type,
              ts,
          }
      }
  }

  fn rand_u32() -> u32 {
      use std::time::{SystemTime, UNIX_EPOCH};
      // 简单伪随机，不需要 crypto-safe
      let nanos = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .unwrap_or_default()
          .subsec_nanos();
      nanos ^ (nanos << 13) ^ (nanos >> 7)
  }

  /// 线程安全消息存储：环形缓冲 + 去重 ID 集合
  pub struct MessageStore {
      messages: Mutex<Vec<Message>>,
      seen_ids: Mutex<Vec<String>>, // 用 Vec 保持顺序，便于裁剪
  }

  impl MessageStore {
      pub fn new() -> Self {
          Self {
              messages: Mutex::new(Vec::with_capacity(MAX_MESSAGES + 1)),
              seen_ids: Mutex::new(Vec::with_capacity(DEDUP_WINDOW + 1)),
          }
      }

      /// 尝试插入消息。返回 false 表示该消息已存在（去重）。
      pub fn insert(&self, msg: Message) -> bool {
          let mut seen = self.seen_ids.lock().unwrap();
          if seen.contains(&msg.id) {
              return false;
          }
          seen.push(msg.id.clone());
          if seen.len() > DEDUP_WINDOW {
              seen.drain(0..seen.len() - DEDUP_WINDOW);
          }
          drop(seen);

          let mut msgs = self.messages.lock().unwrap();
          msgs.push(msg);
          if msgs.len() > MAX_MESSAGES {
              msgs.drain(0..msgs.len() - MAX_MESSAGES);
          }
          true
      }

      pub fn history(&self) -> Vec<Message> {
          self.messages.lock().unwrap().clone()
      }
  }
  ```

- [ ] **Step 2: 在 main.rs 中声明模块（骨架）**

  编辑 `lan-chat/src-tauri/src/main.rs`，确保包含：

  ```rust
  // Prevents additional console window on Windows in release
  #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

  mod messages;

  fn main() {
      tauri::Builder::default()
          .run(tauri::generate_context!())
          .expect("error while running tauri application");
  }
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error（可能有 unused warning，忽略）。

- [ ] **Step 4: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/messages.rs src-tauri/src/main.rs
  git commit -m "feat: add message store with ring buffer and dedup"
  ```

---

## Task 3: network.rs — 网卡枚举与智能过滤

**Files:**
- Create: `lan-chat/src-tauri/src/network.rs`

- [ ] **Step 1: 创建 network.rs**

  ```rust
  // lan-chat/src-tauri/src/network.rs
  use serde::{Deserialize, Serialize};
  use std::net::IpAddr;

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct NetworkInterface {
      pub name: String,
      pub ip: String,
      pub enabled: bool, // 用户是否选中
  }

  /// 硬过滤：名称前缀黑名单（VPN、虚拟网桥、Docker 等）
  const HARD_EXCLUDE_PREFIXES: &[&str] = &[
      "lo",        // loopback
      "tun",       // OpenVPN, WireGuard tun
      "utun",      // macOS VPN tunnels
      "wg",        // WireGuard
      "ppp",       // PPP
      "ipsec",     // IPSec
      "tailscale", // Tailscale
      "docker",    // Docker bridge
      "br-",       // Docker/LXC named bridges
      "virbr",     // libvirt
      "veth",      // Docker veth pair
      "vmnet",     // VMware
      "vboxnet",   // VirtualBox
      "pvnet",     // Parallels
      "llw",       // macOS low latency WLAN (internal)
      "awdl",      // Apple Wireless Direct Link (AirDrop)
      "utun",      // macOS utun (already listed, belt+suspenders)
  ];

  /// 链路本地地址前缀（169.254.x.x）
  const LINK_LOCAL_PREFIX: [u8; 2] = [169, 254];

  pub fn get_candidate_interfaces() -> Vec<NetworkInterface> {
      let Ok(ifaces) = if_addrs::get_if_addrs() else { return vec![] };

      let mut result = Vec::new();

      for iface in ifaces {
          // 仅 IPv4
          let ip = match iface.ip() {
              IpAddr::V4(v4) => v4,
              _ => continue,
          };

          let name = iface.name.clone();

          // 硬过滤：loopback
          if ip.is_loopback() {
              continue;
          }

          // 硬过滤：链路本地
          let octets = ip.octets();
          if octets[0] == LINK_LOCAL_PREFIX[0] && octets[1] == LINK_LOCAL_PREFIX[1] {
              continue;
          }

          // 硬过滤：名称前缀黑名单
          let name_lower = name.to_lowercase();
          if HARD_EXCLUDE_PREFIXES
              .iter()
              .any(|prefix| name_lower.starts_with(prefix))
          {
              continue;
          }

          // 仅保留私有 IP 段（RFC 1918）
          if !is_private_ipv4(&ip) {
              continue;
          }

          result.push(NetworkInterface {
              name,
              ip: ip.to_string(),
              enabled: true, // 默认全选
          });
      }

      // 去重（同一网卡可能有多个 IP，都保留）
      result.dedup_by(|a, b| a.name == b.name && a.ip == b.ip);
      result
  }

  fn is_private_ipv4(ip: &std::net::Ipv4Addr) -> bool {
      let o = ip.octets();
      // 10.0.0.0/8
      if o[0] == 10 { return true; }
      // 172.16.0.0/12
      if o[0] == 172 && (16..=31).contains(&o[1]) { return true; }
      // 192.168.0.0/16
      if o[0] == 192 && o[1] == 168 { return true; }
      false
  }
  ```

- [ ] **Step 2: 在 main.rs 添加模块声明**

  ```rust
  mod messages;
  mod network;
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error。

- [ ] **Step 4: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/network.rs src-tauri/src/main.rs
  git commit -m "feat: add network interface enumeration with smart filtering"
  ```

---

## Task 4: peers.rs — P2P 连接池

**Files:**
- Create: `lan-chat/src-tauri/src/peers.rs`

- [ ] **Step 1: 创建 peers.rs**

  ```rust
  // lan-chat/src-tauri/src/peers.rs
  //
  // 管理所有 WebSocket peer 连接（出站 + 入站）。
  // 每个 peer 用 node_id (UUID) 唯一标识，同一 node_id 不重复连接。

  use dashmap::DashMap;
  use futures::SinkExt;
  use serde::{Deserialize, Serialize};
  use std::sync::Arc;
  use tokio::sync::mpsc;
  use tokio_tungstenite::tungstenite::Message as WsMessage;

  /// 每个 peer 的元数据
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PeerInfo {
      pub node_id: String,
      pub nickname: String,
      pub addr: String, // "ip:port"
  }

  /// 发送到 peer 的命令
  #[derive(Debug)]
  pub enum PeerCmd {
      Send(String),  // JSON 字符串
      Close,
  }

  /// 全局 Peer 连接池
  pub struct PeerPool {
      /// node_id -> sender channel（发消息给该 peer 的 WebSocket 写端）
      senders: DashMap<String, mpsc::UnboundedSender<PeerCmd>>,
      /// node_id -> PeerInfo
      infos: DashMap<String, PeerInfo>,
  }

  impl PeerPool {
      pub fn new() -> Arc<Self> {
          Arc::new(Self {
              senders: DashMap::new(),
              infos: DashMap::new(),
          })
      }

      /// 注册一个新 peer（连接建立后调用）
      pub fn add(&self, info: PeerInfo, tx: mpsc::UnboundedSender<PeerCmd>) {
          self.senders.insert(info.node_id.clone(), tx);
          self.infos.insert(info.node_id.clone(), info);
      }

      /// 断开并移除一个 peer
      pub fn remove(&self, node_id: &str) {
          if let Some((_, tx)) = self.senders.remove(node_id) {
              let _ = tx.send(PeerCmd::Close);
          }
          self.infos.remove(node_id);
      }

      /// 是否已连接该 node_id
      pub fn contains(&self, node_id: &str) -> bool {
          self.senders.contains_key(node_id)
      }

      /// 向所有 peer 广播 JSON 消息（排除 exclude_node_id，防环路）
      pub fn broadcast(&self, json: &str, exclude_node_id: Option<&str>) {
          for entry in self.senders.iter() {
              if let Some(excl) = exclude_node_id {
                  if entry.key() == excl {
                      continue;
                  }
              }
              let _ = entry.value().send(PeerCmd::Send(json.to_string()));
          }
      }

      /// 获取所有 peer 信息列表（供前端展示）
      pub fn list(&self) -> Vec<PeerInfo> {
          self.infos.iter().map(|e| e.value().clone()).collect()
      }

      pub fn count(&self) -> usize {
          self.senders.len()
      }
  }

  /// WebSocket 写任务：从 channel 收消息，写到 WS sink
  pub async fn run_ws_writer<S>(
      mut sink: S,
      mut rx: mpsc::UnboundedReceiver<PeerCmd>,
  ) where
      S: futures::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error>
          + Unpin
          + Send
          + 'static,
  {
      while let Some(cmd) = rx.recv().await {
          match cmd {
              PeerCmd::Send(json) => {
                  if sink.send(WsMessage::Text(json.into())).await.is_err() {
                      break;
                  }
              }
              PeerCmd::Close => {
                  let _ = sink.send(WsMessage::Close(None)).await;
                  break;
              }
          }
      }
  }
  ```

- [ ] **Step 2: 在 main.rs 添加模块声明**

  ```rust
  mod messages;
  mod network;
  mod peers;
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error。

- [ ] **Step 4: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/peers.rs src-tauri/src/main.rs
  git commit -m "feat: add P2P peer connection pool"
  ```

---

## Task 5: server.rs — Axum WebSocket 服务器（接受入站连接）

**Files:**
- Create: `lan-chat/src-tauri/src/server.rs`

- [ ] **Step 1: 创建 server.rs**

  ```rust
  // lan-chat/src-tauri/src/server.rs
  //
  // 在本机启动 Axum HTTP 服务器，接受其他节点的 WebSocket 入站连接。
  // 握手协议：
  //   1. 连接建立后，双方互发 {"type":"hello","node_id":"...","nickname":"...","version":"1"}
  //   2. 之后正常收发 Message JSON

  use crate::messages::{Message, MessageStore};
  use crate::peers::{PeerInfo, PeerPool, PeerCmd, run_ws_writer};
  use axum::{
      extract::{ws::{WebSocket, WebSocketUpgrade, Message as AxumWsMsg}, State},
      response::IntoResponse,
      routing::get,
      Router,
  };
  use futures::{StreamExt, SinkExt};
  use serde::{Deserialize, Serialize};
  use std::net::SocketAddr;
  use std::sync::Arc;
  use tauri::AppHandle;
  use tokio::sync::mpsc;

  #[derive(Debug, Serialize, Deserialize)]
  struct HelloMsg {
      #[serde(rename = "type")]
      msg_type: String,
      node_id: String,
      nickname: String,
      version: String,
  }

  #[derive(Clone)]
  pub struct ServerState {
      pub pool: Arc<PeerPool>,
      pub store: Arc<MessageStore>,
      pub node_id: String,
      pub nickname: Arc<tokio::sync::RwLock<String>>,
      pub app: AppHandle,
  }

  /// 启动 WebSocket 服务器，尝试端口 4242..4252
  /// 返回实际绑定的端口
  pub async fn start_server(state: ServerState) -> anyhow::Result<u16> {
      let port = find_free_port(4242, 4252).await?;
      let addr = SocketAddr::from(([0, 0, 0, 0], port));

      let app = Router::new()
          .route("/ws", get(ws_handler))
          .with_state(state);

      let listener = tokio::net::TcpListener::bind(addr).await?;
      tracing::info!("WebSocket server listening on {}", addr);

      tokio::spawn(async move {
          axum::serve(listener, app).await.unwrap();
      });

      Ok(port)
  }

  async fn find_free_port(start: u16, end: u16) -> anyhow::Result<u16> {
      for port in start..=end {
          if tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port)))
              .await
              .is_ok()
          {
              return Ok(port);
          }
      }
      anyhow::bail!("No free port found in range {}..{}", start, end)
  }

  async fn ws_handler(
      ws: WebSocketUpgrade,
      State(state): State<ServerState>,
  ) -> impl IntoResponse {
      ws.on_upgrade(move |socket| handle_inbound(socket, state))
  }

  async fn handle_inbound(socket: WebSocket, state: ServerState) {
      let (mut sink, mut stream) = socket.split();

      // 1. 发送 hello
      let my_nick = state.nickname.read().await.clone();
      let hello = serde_json::to_string(&HelloMsg {
          msg_type: "hello".into(),
          node_id: state.node_id.clone(),
          nickname: my_nick,
          version: "1".into(),
      })
      .unwrap();
      if sink.send(AxumWsMsg::Text(hello.into())).await.is_err() {
          return;
      }

      // 2. 等待对方 hello
      let peer_info = loop {
          match stream.next().await {
              Some(Ok(AxumWsMsg::Text(t))) => {
                  if let Ok(h) = serde_json::from_str::<HelloMsg>(&t) {
                      if h.msg_type == "hello" {
                          break PeerInfo {
                              node_id: h.node_id,
                              nickname: h.nickname,
                              addr: "inbound".into(),
                          };
                      }
                  }
              }
              _ => return,
          }
      };

      // 3. 防止自连 & 重复连接
      if peer_info.node_id == state.node_id || state.pool.contains(&peer_info.node_id) {
          return;
      }

      // 4. 发送历史消息
      let history = state.store.history();
      if !history.is_empty() {
          let hist_json = serde_json::to_string(&serde_json::json!({
              "type": "history",
              "messages": history
          }))
          .unwrap();
          if sink.send(AxumWsMsg::Text(hist_json.into())).await.is_err() {
              return;
          }
      }

      // 5. 注册 peer，启动写任务
      let (tx, rx) = mpsc::unbounded_channel::<PeerCmd>();
      // 将 axum sink 转换为 tungstenite-compatible sink via wrapper
      let node_id = peer_info.node_id.clone();
      state.pool.add(peer_info.clone(), tx);

      // 通知前端 peer 列表更新
      emit_peer_update(&state);

      // 写任务：直接用 axum ws sink（axum Message 和 tungstenite Message 不同，用 channel 桥接）
      let pool_clone = state.pool.clone();
      let node_id_clone = node_id.clone();
      let state_clone = state.clone();

      tokio::spawn(async move {
          run_axum_writer(sink, rx).await;
          pool_clone.remove(&node_id_clone);
          emit_peer_update(&state_clone);
      });

      // 读任务：接收对方消息
      while let Some(Ok(msg)) = stream.next().await {
          if let AxumWsMsg::Text(t) = msg {
              handle_incoming_message(&t, &node_id, &state);
          }
      }
  }

  fn handle_incoming_message(json: &str, from_node_id: &str, state: &ServerState) {
      if let Ok(msg) = serde_json::from_str::<Message>(json) {
          if state.store.insert(msg.clone()) {
              // 广播给其他 peer（排除发送方，防环路）
              state.pool.broadcast(json, Some(from_node_id));
              // 推送到前端
              let _ = state.app.emit("message", &msg);
          }
      }
  }

  fn emit_peer_update(state: &ServerState) {
      let peers = state.pool.list();
      let _ = state.app.emit("peer_update", &peers);
  }

  /// axum WebSocket sink 的写任务（axum 用自己的 Message 类型）
  async fn run_axum_writer(
      mut sink: impl futures::Sink<AxumWsMsg, Error = axum::Error> + Unpin,
      mut rx: mpsc::UnboundedReceiver<PeerCmd>,
  ) {
      while let Some(cmd) = rx.recv().await {
          match cmd {
              PeerCmd::Send(json) => {
                  if sink.send(AxumWsMsg::Text(json.into())).await.is_err() {
                      break;
                  }
              }
              PeerCmd::Close => {
                  let _ = sink.send(AxumWsMsg::Close(None)).await;
                  break;
              }
          }
      }
  }
  ```

- [ ] **Step 2: 在 main.rs 添加模块声明**

  ```rust
  mod messages;
  mod network;
  mod peers;
  mod server;
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error（可能有 unused imports warning，忽略）。

- [ ] **Step 4: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/server.rs src-tauri/src/main.rs
  git commit -m "feat: add axum WebSocket server for inbound peer connections"
  ```

---

## Task 6: mdns.rs — mDNS 广播与发现

**Files:**
- Create: `lan-chat/src-tauri/src/mdns.rs`

- [ ] **Step 1: 创建 mdns.rs**

  ```rust
  // lan-chat/src-tauri/src/mdns.rs
  //
  // 使用 mdns-sd crate 在选定网卡上广播本节点，并发现其他 _lanchat._tcp.local. 节点。
  // 发现新节点后，尝试建立 WebSocket 出站连接。

  use crate::peers::{PeerInfo, PeerPool, PeerCmd};
  use crate::messages::{Message, MessageStore};
  use crate::server::ServerState;
  use futures::StreamExt;
  use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
  use serde::{Deserialize, Serialize};
  use std::collections::HashMap;
  use std::sync::Arc;
  use tauri::AppHandle;
  use tokio::sync::mpsc;
  use tokio_tungstenite::{connect_async, tungstenite::Message as WsMsg};

  const SERVICE_TYPE: &str = "_lanchat._tcp.local.";

  #[derive(Debug, Serialize, Deserialize)]
  struct HelloMsg {
      #[serde(rename = "type")]
      msg_type: String,
      node_id: String,
      nickname: String,
      version: String,
  }

  /// 启动 mDNS 服务：广播自身 + 后台发现任务
  pub fn start_mdns(
      node_id: String,
      nickname: Arc<tokio::sync::RwLock<String>>,
      port: u16,
      iface_ips: Vec<String>, // 选定网卡的 IP 列表
      state: ServerState,
  ) -> anyhow::Result<()> {
      let mdns = ServiceDaemon::new()?;

      // 为每个选定 IP 注册服务
      for ip in &iface_ips {
          let mut props = HashMap::new();
          props.insert("node_id".to_string(), node_id.clone());

          let service_name = format!("lanchat-{}", &node_id[..8]);
          let instance_name = format!("{}-{}", service_name, ip.replace('.', "-"));

          let service = ServiceInfo::new(
              SERVICE_TYPE,
              &instance_name,
              &format!("{}.local.", hostname()),
              ip.as_str(),
              port,
              Some(props),
          )?;

          mdns.register(service)?;
          tracing::info!("mDNS registered on {}", ip);
      }

      // 启动发现任务
      let receiver = mdns.browse(SERVICE_TYPE)?;
      let node_id_clone = node_id.clone();
      let state_clone = state.clone();

      tokio::spawn(async move {
          while let Ok(event) = receiver.recv_async().await {
              match event {
                  ServiceEvent::ServiceResolved(info) => {
                      let remote_node_id = info
                          .get_properties()
                          .get_property_val_str("node_id")
                          .unwrap_or("")
                          .to_string();

                      // 过滤自身
                      if remote_node_id == node_id_clone || remote_node_id.is_empty() {
                          continue;
                      }

                      // 已连接则跳过
                      if state_clone.pool.contains(&remote_node_id) {
                          continue;
                      }

                      // 取第一个地址
                      let addrs: Vec<_> = info.get_addresses().iter().cloned().collect();
                      let Some(addr) = addrs.first() else { continue };
                      let port = info.get_port();
                      let url = format!("ws://{}:{}/ws", addr, port);

                      tracing::info!("mDNS discovered peer {} at {}", remote_node_id, url);

                      let state_for_conn = state_clone.clone();
                      let nid = remote_node_id.clone();
                      let addr_str = format!("{}:{}", addr, port);

                      tokio::spawn(async move {
                          connect_to_peer(url, addr_str, nid, state_for_conn).await;
                      });
                  }
                  ServiceEvent::ServiceRemoved(_, full_name) => {
                      tracing::info!("mDNS peer removed: {}", full_name);
                      // 连接断开由 WebSocket 读任务检测，这里不强制断开
                  }
                  _ => {}
              }
          }
      });

      Ok(())
  }

  /// 出站连接：连接到已发现的 peer
  pub async fn connect_to_peer(
      url: String,
      addr_str: String,
      expected_node_id: String,
      state: ServerState,
  ) {
      let Ok((ws_stream, _)) = connect_async(&url).await else {
          tracing::warn!("Failed to connect to peer at {}", url);
          return;
      };

      let (mut sink, mut stream) = ws_stream.split();

      // 1. 发送 hello
      let my_nick = state.nickname.read().await.clone();
      let hello = serde_json::to_string(&HelloMsg {
          msg_type: "hello".into(),
          node_id: state.node_id.clone(),
          nickname: my_nick,
          version: "1".into(),
      })
      .unwrap();

      if sink.send(WsMsg::Text(hello.into())).await.is_err() {
          return;
      }

      // 2. 等待对方 hello
      let peer_info = loop {
          match stream.next().await {
              Some(Ok(WsMsg::Text(t))) => {
                  if let Ok(h) = serde_json::from_str::<HelloMsg>(&t) {
                      if h.msg_type == "hello" {
                          break PeerInfo {
                              node_id: h.node_id,
                              nickname: h.nickname,
                              addr: addr_str.clone(),
                          };
                      }
                  }
                  // history 消息在 hello 之后，先忽略
              }
              _ => return,
          }
      };

      // 3. 再次检查去重（防止竞态）
      if peer_info.node_id == state.node_id || state.pool.contains(&peer_info.node_id) {
          return;
      }

      // 4. 注册 peer
      let (tx, rx) = mpsc::unbounded_channel::<PeerCmd>();
      let node_id = peer_info.node_id.clone();
      state.pool.add(peer_info, tx);
      emit_peer_update(&state);

      // 5. 启动写任务
      let pool_clone = state.pool.clone();
      let node_id_clone = node_id.clone();
      let state_clone = state.clone();

      tokio::spawn(async move {
          crate::peers::run_ws_writer(sink, rx).await;
          pool_clone.remove(&node_id_clone);
          emit_peer_update(&state_clone);
      });

      // 6. 读任务：处理对方推来的消息（history + message）
      while let Some(Ok(msg)) = stream.next().await {
          if let WsMsg::Text(t) = msg {
              handle_peer_message(&t, &node_id, &state);
          }
      }
  }

  fn handle_peer_message(json: &str, from_node_id: &str, state: &ServerState) {
      // 尝试解析为带 type 字段的包
      if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
          match v.get("type").and_then(|t| t.as_str()) {
              Some("history") => {
                  if let Some(arr) = v.get("messages").and_then(|m| m.as_array()) {
                      for item in arr {
                          if let Ok(msg) = serde_json::from_value::<Message>(item.clone()) {
                              if state.store.insert(msg.clone()) {
                                  let _ = state.app.emit("message", &msg);
                              }
                          }
                      }
                  }
                  return;
              }
              _ => {}
          }
      }

      // 普通消息
      if let Ok(msg) = serde_json::from_str::<Message>(json) {
          if state.store.insert(msg.clone()) {
              state.pool.broadcast(json, Some(from_node_id));
              let _ = state.app.emit("message", &msg);
          }
      }
  }

  fn emit_peer_update(state: &ServerState) {
      let peers = state.pool.list();
      let _ = state.app.emit("peer_update", &peers);
  }

  fn hostname() -> String {
      std::env::var("HOSTNAME")
          .or_else(|_| {
              std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string())
          })
          .unwrap_or_else(|_| "localhost".to_string())
  }
  ```

- [ ] **Step 2: 在 main.rs 添加模块声明**

  ```rust
  mod messages;
  mod network;
  mod peers;
  mod server;
  mod mdns;
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error。

- [ ] **Step 4: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/mdns.rs src-tauri/src/main.rs
  git commit -m "feat: add mDNS service discovery and outbound peer connections"
  ```

---

## Task 7: commands.rs — Tauri IPC Commands

**Files:**
- Create: `lan-chat/src-tauri/src/commands.rs`

- [ ] **Step 1: 创建 commands.rs**

  ```rust
  // lan-chat/src-tauri/src/commands.rs
  //
  // 所有前端可调用的 Tauri IPC commands。

  use crate::messages::{Message, MsgType, MessageStore};
  use crate::network::{get_candidate_interfaces, NetworkInterface};
  use crate::peers::{PeerInfo, PeerPool};
  use crate::mdns::connect_to_peer;
  use crate::server::ServerState;
  use std::sync::Arc;
  use tauri::State;

  /// 应用全局状态（注册到 Tauri manage）
  pub struct AppState {
      pub server_state: ServerState,
      pub port: u16,
  }

  // ── 消息 ──────────────────────────────────────────────

  /// 发送消息：存入本地 store，广播给所有 peer，推送给前端
  #[tauri::command]
  pub async fn send_message(
      text: String,
      msg_type: String,
      state: State<'_, AppState>,
  ) -> Result<(), String> {
      let nick = state.server_state.nickname.read().await.clone();
      let t = if msg_type == "clipboard" {
          MsgType::Clipboard
      } else {
          MsgType::Text
      };
      let msg = Message::new(text, nick, t);
      let json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;

      state.server_state.store.insert(msg.clone());
      state.server_state.pool.broadcast(&json, None);
      let _ = state.server_state.app.emit("message", &msg);
      Ok(())
  }

  /// 获取历史消息（新窗口/刷新时调用）
  #[tauri::command]
  pub fn get_history(state: State<'_, AppState>) -> Vec<Message> {
      state.server_state.store.history()
  }

  // ── Peer 管理 ──────────────────────────────────────────

  /// 手动连接 peer（用户输入 IP）
  #[tauri::command]
  pub async fn connect_peer(
      ip: String,
      port: Option<u16>,
      state: State<'_, AppState>,
  ) -> Result<(), String> {
      let p = port.unwrap_or(4242);
      let url = format!("ws://{}:{}/ws", ip, p);
      let addr_str = format!("{}:{}", ip, p);
      let server_state = state.server_state.clone();
      let node_id = "manual".to_string(); // 握手后会用真实 node_id

      tokio::spawn(async move {
          connect_to_peer(url, addr_str, node_id, server_state).await;
      });
      Ok(())
  }

  /// 获取当前 peer 列表
  #[tauri::command]
  pub fn get_peers(state: State<'_, AppState>) -> Vec<PeerInfo> {
      state.server_state.pool.list()
  }

  // ── 网络 ──────────────────────────────────────────────

  /// 获取候选网卡列表
  #[tauri::command]
  pub fn get_interfaces() -> Vec<NetworkInterface> {
      get_candidate_interfaces()
  }

  /// 获取本机监听端口
  #[tauri::command]
  pub fn get_port(state: State<'_, AppState>) -> u16 {
      state.port
  }

  // ── 设置 ──────────────────────────────────────────────

  /// 更新昵称
  #[tauri::command]
  pub async fn set_nickname(
      nickname: String,
      state: State<'_, AppState>,
  ) -> Result<(), String> {
      let nick = nickname.trim().chars().take(40).collect::<String>();
      *state.server_state.nickname.write().await = nick;
      Ok(())
  }

  /// 获取当前昵称
  #[tauri::command]
  pub async fn get_nickname(state: State<'_, AppState>) -> String {
      state.server_state.nickname.read().await.clone()
  }
  ```

- [ ] **Step 2: 在 main.rs 添加模块声明**

  ```rust
  mod commands;
  mod messages;
  mod mdns;
  mod network;
  mod peers;
  mod server;
  ```

- [ ] **Step 3: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error。

- [ ] **Step 4: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/commands.rs src-tauri/src/main.rs
  git commit -m "feat: add Tauri IPC commands for frontend communication"
  ```

---

## Task 8: main.rs — 完整组装入口

**Files:**
- Modify: `lan-chat/src-tauri/src/main.rs`

- [ ] **Step 1: 完整覆盖 main.rs**

  ```rust
  // lan-chat/src-tauri/src/main.rs
  #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

  mod commands;
  mod mdns;
  mod messages;
  mod network;
  mod peers;
  mod server;

  use commands::AppState;
  use messages::MessageStore;
  use network::get_candidate_interfaces;
  use peers::PeerPool;
  use server::{ServerState, start_server};
  use std::sync::Arc;
  use tauri::Manager;
  use tokio::sync::RwLock;

  fn main() {
      tracing_subscriber::fmt()
          .with_env_filter(
              tracing_subscriber::EnvFilter::try_from_default_env()
                  .unwrap_or_else(|_| "lan_chat=info".parse().unwrap()),
          )
          .init();

      tauri::Builder::default()
          .plugin(tauri_plugin_shell::init())
          .invoke_handler(tauri::generate_handler![
              commands::send_message,
              commands::get_history,
              commands::connect_peer,
              commands::get_peers,
              commands::get_interfaces,
              commands::get_port,
              commands::set_nickname,
              commands::get_nickname,
          ])
          .setup(|app| {
              let handle = app.handle().clone();

              // 生成或读取持久 node_id（简单起见，每次启动随机，后续可持久化）
              let node_id = uuid::Uuid::new_v4().to_string();
              let nickname = Arc::new(RwLock::new(format!("Device-{}", &node_id[..4])));
              let pool = PeerPool::new();
              let store = Arc::new(MessageStore::new());

              let server_state = ServerState {
                  pool: pool.clone(),
                  store: store.clone(),
                  node_id: node_id.clone(),
                  nickname: nickname.clone(),
                  app: handle.clone(),
              };

              let server_state_clone = server_state.clone();

              tauri::async_runtime::spawn(async move {
                  // 启动 WebSocket 服务器
                  let port = match start_server(server_state_clone.clone()).await {
                      Ok(p) => p,
                      Err(e) => {
                          tracing::error!("Failed to start server: {}", e);
                          return;
                      }
                  };

                  tracing::info!("Server started on port {}", port);

                  // 注册 AppState（需要 port）
                  handle.manage(AppState {
                      server_state: server_state_clone.clone(),
                      port,
                  });

                  // 获取候选网卡 IP
                  let iface_ips: Vec<String> = get_candidate_interfaces()
                      .into_iter()
                      .filter(|i| i.enabled)
                      .map(|i| i.ip)
                      .collect();

                  tracing::info!("Broadcasting on interfaces: {:?}", iface_ips);

                  // 启动 mDNS
                  if let Err(e) = mdns::start_mdns(
                      node_id,
                      nickname,
                      port,
                      iface_ips,
                      server_state_clone,
                  ) {
                      tracing::warn!("mDNS failed to start: {}", e);
                  }
              });

              Ok(())
          })
          .run(tauri::generate_context!())
          .expect("error while running tauri application");
  }
  ```

- [ ] **Step 2: 验证编译**

  ```bash
  cd lan-chat/src-tauri
  cargo check
  ```

  期望：无 error。

- [ ] **Step 3: Commit**

  ```bash
  cd lan-chat
  git add src-tauri/src/main.rs
  git commit -m "feat: wire up main.rs — server + mDNS + Tauri commands"
  ```

---

## Task 9: 前端改造 — 移除 Socket.IO，接入 Tauri IPC

**Files:**
- Modify: `lan-chat/src/index.html`（基于现有 `public/index.html` 改造）

- [ ] **Step 1: 复制现有前端到 src/（Tauri 会从这里加载）**

  ```bash
  cp lan-chat/public/index.html lan-chat/src/index.html
  ```

- [ ] **Step 2: 移除 Socket.IO script 标签**

  在 `src/index.html` 中，删除这一行：

  ```html
  <script src="/socket.io/socket.io.js"></script>
  ```

- [ ] **Step 3: 替换整个 JS 通信层**

  在 `src/index.html` 的 `<script>` 区域（原先有 `const socket = io()` 的地方），
  找到并替换通信相关代码为以下 Tauri IPC 版本。

  **找到原始初始化块（类似）：**
  ```js
  const socket = io();
  socket.on('history', (msgs) => { /* ... */ });
  socket.on('message', (msg) => { /* ... */ });
  ```

  **替换为：**
  ```js
  // ── Tauri IPC bridge ──────────────────────────────────
  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;

  // 监听实时消息推送
  listen('message', (event) => {
    appendMessage(event.payload);
  });

  // 监听 peer 列表更新
  listen('peer_update', (event) => {
    updatePeerStatus(event.payload);
  });

  // 加载历史消息
  async function loadHistory() {
    const msgs = await invoke('get_history');
    msgs.forEach(appendMessage);
  }

  // 加载本机信息
  async function loadLocalInfo() {
    const port = await invoke('get_port');
    const ifaces = await invoke('get_interfaces');
    const nickname = await invoke('get_nickname');
    updateLocalInfo({ port, ifaces, nickname });
  }

  // 应用启动
  document.addEventListener('DOMContentLoaded', async () => {
    await loadHistory();
    await loadLocalInfo();
  });
  ```

  **找到原始发送消息的代码（类似）：**
  ```js
  socket.emit('message', { text, device, type: 'text' });
  ```

  **替换为：**
  ```js
  await invoke('send_message', { text, msgType: 'text' });
  ```

  **剪贴板发送（类似）：**
  ```js
  socket.emit('message', { text, device, type: 'clipboard' });
  ```

  **替换为：**
  ```js
  await invoke('send_message', { text, msgType: 'clipboard' });
  ```

  > **注意：** 原来的 `device` 字段由后端从 `nickname` 读取，前端不再需要传 device。

- [ ] **Step 4: 新增设置面板 HTML（在 body 内追加）**

  在 `</body>` 前添加：

  ```html
  <!-- 设置/状态面板 -->
  <div id="info-panel" style="
    position:fixed; bottom:0; left:0; right:0;
    background:#111211; border-top:1px solid #1F201D;
    padding:8px 16px; font-size:12px; color:#5C5D54;
    display:flex; gap:16px; align-items:center;
    font-family: 'JetBrains Mono', monospace;
  ">
    <span id="peer-count">● 0 peers</span>
    <span id="local-ips">IP: --</span>
    <input id="nickname-input" placeholder="昵称" style="
      background:transparent; border:1px solid #1F201D; color:#D9D6CC;
      padding:2px 6px; font-size:12px; font-family:inherit; border-radius:3px;
    " />
    <button id="nickname-btn" style="
      background:#1F201D; color:#D9D6CC; border:none; padding:2px 8px;
      font-size:12px; cursor:pointer; border-radius:3px;
    ">改名</button>
    <span style="flex:1"></span>
    <input id="manual-ip" placeholder="手动连接 IP" style="
      background:transparent; border:1px solid #1F201D; color:#D9D6CC;
      padding:2px 6px; font-size:12px; font-family:inherit; border-radius:3px; width:140px;
    " />
    <button id="manual-connect-btn" style="
      background:#1F201D; color:#7CFFB2; border:none; padding:2px 8px;
      font-size:12px; cursor:pointer; border-radius:3px;
    ">连接</button>
  </div>
  ```

- [ ] **Step 5: 新增面板交互 JS**

  在步骤 3 的脚本之后追加：

  ```js
  // ── 状态面板 ──────────────────────────────────────────
  function updatePeerStatus(peers) {
    const count = Array.isArray(peers) ? peers.length : 0;
    document.getElementById('peer-count').textContent =
      `● ${count} peer${count !== 1 ? 's' : ''}`;
  }

  function updateLocalInfo({ port, ifaces, nickname }) {
    const ips = ifaces.map(i => `${i.ip}:${port}`).join('  ');
    document.getElementById('local-ips').textContent = `IP: ${ips || '--'}`;
    document.getElementById('nickname-input').value = nickname;
  }

  // 改名
  document.getElementById('nickname-btn').addEventListener('click', async () => {
    const nick = document.getElementById('nickname-input').value.trim();
    if (nick) await invoke('set_nickname', { nickname: nick });
  });

  // 手动连接
  document.getElementById('manual-connect-btn').addEventListener('click', async () => {
    const raw = document.getElementById('manual-ip').value.trim();
    if (!raw) return;
    const [ip, portStr] = raw.split(':');
    const port = portStr ? parseInt(portStr) : undefined;
    await invoke('connect_peer', { ip, port });
    document.getElementById('manual-ip').value = '';
  });
  ```

- [ ] **Step 6: 验证前端文件存在**

  ```bash
  ls lan-chat/src/index.html
  ```

  期望：文件存在。

- [ ] **Step 7: Commit**

  ```bash
  cd lan-chat
  git add src/index.html
  git commit -m "feat: migrate frontend from Socket.IO to Tauri IPC"
  ```

---

## Task 10: 首次构建与调试运行

**Files:**
- 无新文件，验证整体可构建和运行

- [ ] **Step 1: 安装前端依赖（仅 Tauri CLI）**

  ```bash
  cd lan-chat
  npm install
  ```

- [ ] **Step 2: 尝试开发模式运行**

  ```bash
  cd lan-chat
  cargo tauri dev
  ```

  期望：应用窗口弹出，控制台无 panic。

  若出现编译错误，根据错误信息修复对应文件。常见问题：
  - `AppState` 未实现 `Send`：确认所有 Arc 字段 OK
  - mDNS 在 Linux 需要 avahi 服务运行：`sudo systemctl start avahi-daemon`
  - axum WebSocket feature 未启用：确认 Cargo.toml 中 `axum = { features = ["ws"] }`

- [ ] **Step 3: 验证基本功能**

  - 窗口打开，显示空消息列表
  - 底部状态栏显示本机 IP
  - 输入消息并发送，消息出现在列表

- [ ] **Step 4: 在两台设备上测试（或同一台开两个实例验证）**

  同一台机器开两个终端，各跑 `cargo tauri dev`（端口会自动 +1）：
  ```bash
  # 终端 1
  cd lan-chat && cargo tauri dev

  # 终端 2（等终端 1 启动后）
  cd lan-chat && cargo tauri dev
  ```

  期望：两个窗口 peer count 变为 1，发消息对方能收到。

- [ ] **Step 5: Commit**

  ```bash
  cd lan-chat
  git add -A
  git commit -m "fix: resolve build issues from dev run"
  ```

---

## Task 11: 打包发布构建

**Files:**
- 无新文件

- [ ] **Step 1: Ubuntu 打包**

  ```bash
  cd lan-chat
  cargo tauri build
  ```

  产物位于：
  - `src-tauri/target/release/bundle/deb/lan-chat_1.0.0_amd64.deb`
  - `src-tauri/target/release/bundle/appimage/lan-chat_1.0.0_amd64.AppImage`

- [ ] **Step 2: macOS 打包**（在 Mac 上执行）

  ```bash
  cd lan-chat
  cargo tauri build
  ```

  产物位于：
  - `src-tauri/target/release/bundle/dmg/LAN Chat_1.0.0_aarch64.dmg`
  - `src-tauri/target/release/bundle/macos/LAN Chat.app`

- [ ] **Step 3: 验证安装包可安装**

  Ubuntu：
  ```bash
  sudo dpkg -i src-tauri/target/release/bundle/deb/lan-chat_1.0.0_amd64.deb
  lan-chat
  ```

  macOS：双击 `.dmg` 安装，从 Applications 启动。

- [ ] **Step 4: 最终 Commit**

  ```bash
  cd lan-chat
  git add -A
  git commit -m "chore: verify release build on Ubuntu and macOS"
  ```

---

## 附：常见问题排查

| 问题 | 原因 | 解决 |
|------|------|------|
| mDNS 发现不到对方 | 防火墙拦截 UDP 5353 | `sudo ufw allow 5353/udp` |
| 端口冲突 | 4242 被占用 | 自动递增到 4252，查看日志确认实际端口 |
| WebKitGTK 缺失 | Ubuntu 缺系统依赖 | 运行前置检查中的 apt 命令 |
| avahi-daemon 未运行 | mDNS 在 Linux 依赖 avahi | `sudo systemctl enable --now avahi-daemon` |
| VPN 干扰发现 | tun 接口被正确过滤，但 VPN 改变路由 | 用手动 IP 连接兜底 |
| 雷电网桥发现不到 | 桥接接口名非标准 | 在状态栏"手动连接"框输入对方 IP 直连 |
