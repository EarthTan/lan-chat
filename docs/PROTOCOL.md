# Protocol

> LAN Chat 的 WebSocket 协议和 Tauri IPC 接口。

## 1. 传输层

- **WebSocket** over plain HTTP（无 TLS），端口默认 `4242`（自动尝试 4242-4252）
- **路径**：`/ws`
- **消息格式**：所有 WebSocket frame 都是 UTF-8 JSON
- **mDNS 服务类型**：`_lanchat._tcp.local.`

## 2. WebSocket 协议

### 2.1 握手：hello

连接建立后（无论客户端还是服务端接受方），**第一帧**必须是 `hello`：

```json
{
  "type": "hello",
  "node_id": "550e8400-e29b-41d4-a716-446655440000",
  "nickname": "Alice",
  "version": "1"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `type` | string | 固定 `"hello"` |
| `node_id` | string | UUID v4，每次启动重新生成 |
| `nickname` | string | 设备昵称，可通过 `set_nickname` 修改 |
| `version` | string | 协议版本，当前 `"1"` |

收到对端的 `hello` 后，**也回一个 `hello`**，然后才进入正常消息交换。

> ⚠️ 未交换 `hello` 之前收到的消息帧应当丢弃（实现里 `server.rs` 的 `ws_handler` 在 `hello` 之前只允许一条消息通过握手状态机）。

### 2.2 聊天消息

```json
{
  "id": "1719666000000_abc12",
  "text": "hello world",
  "device": "Alice",
  "type": "text",
  "ts": 1719666000000
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | string | `<ts_ms>_<5hex>`，由 `Message::new` 生成 |
| `text` | string | 内容，trim 后最多 8000 字符 |
| `device` | string | 发送方昵称，trim 后最多 40 字符 |
| `type` | string | `"text"` 或 `"clipboard"` |
| `ts` | number (u64) | Unix epoch 毫秒 |

`type` 取值：
- `"text"` — 普通聊天
- `"clipboard"` — 剪贴板共享内容，前端用不同样式（紫色气泡）渲染

### 2.3 消息生命周期

```
send_message(text, type)
  ↓
构造 Message { id, ts }
  ↓
MessageStore::insert   ←─── 去重（500 条 ID 滑动窗口）
  ↓
PeerPool::broadcast(json, exclude=None)   ←── 排除 None
  ↓
所有 peer 的写任务通过 mpsc 收到
  ↓
emit("message", msg)  →  前端
```

### 2.4 转发（防止广播环路）

当节点 B 从节点 A 收到消息后，会：
1. 存到自己 store（去重）
2. 通过 `PeerPool::broadcast(json, exclude=Some(A.node_id))` 转发给 **除 A 之外** 的对端
3. 发给自己的前端

`exclude` 机制 + `MessageStore` 的去重窗口，**两层防护** 防止消息在网状拓扑中无限循环。

## 3. mDNS 协议

### 3.1 服务注册

- **Service type**：`"_lanchat._tcp.local."`
- **Port**：当前 WS 端口
- **Hostname**：`{hostname}.local.`
- **TXT 记录**：`{"node_id": "<uuid>"}`
- **Instance name**：每个接口 IP 一个独立实例，格式 `lanchat-{short_id}-{ip_dashed}`
  - 例：`lanchat-550e8400-192-168-1-42`
  - 短 ID 取 `node_id` 前 8 字符

### 3.2 发现

- 节点启动时调用 `ServiceDaemon::browse("_lanchat._tcp.local.")`
- 收到 `ServiceEvent::ServiceResolved` 时，从 TXT 拿 `node_id`，去重后建立出站 WS 连接
- 收到 `ServiceEvent::ServiceRemoved` 时清理对端

> 浏览器与服务器共用同一个 `ServiceDaemon`。

## 4. Tauri IPC

前端通过 `window.__TAURI__.core.invoke` 同步调用，通过 `window.__TAURI__.event.listen` 订阅事件。

### 4.1 Commands

#### `send_message`

```js
await invoke("send_message", { text: "hi", msgType: "text" });
// msgType: "text" | "clipboard"
```

#### `get_history`

```js
const msgs = await invoke("get_history");
// → Message[]（最近 200 条，按时间升序）
```

#### `get_peers`

```js
const peers = await invoke("get_peers");
// → [{ node_id, nickname, addr }]
```

#### `connect_peer`

```js
await invoke("connect_peer", { ip: "192.168.1.42", port: 4242 });
// port 可选，默认 4242
// 用于 mDNS 不通时手动连接
```

#### `get_interfaces`

```js
const ifaces = await invoke("get_interfaces");
// → [{ name, ip, is_up }]
```

#### `get_port`

```js
const port = await invoke("get_port");
// → number，当前绑定的 WS 端口
```

#### `get_nickname` / `set_nickname`

```js
const nick = await invoke("get_nickname");
await invoke("set_nickname", { nickname: "新名字" });
```

### 4.2 Events

#### `message`

```js
import { listen } from "@tauri-apps/api/event";
await listen("message", (event) => {
    const msg = event.payload;  // Message
    // ...
});
```

每次有新消息（本地发送或远端收到）都会触发。

## 5. 示例：完整交互

```
[A 启动]                 [B 启动]
  │                         │
  ├─ bind :4242             ├─ bind :4242
  ├─ mDNS register A        ├─ mDNS register B
  │                         │
  │   mDNS resolve B  ──────┤
  │                         │
  ├──── WS upgrade ─────────>
  │                         │
  ├─ hello (A) ────────────>├─ 收到 A
  │                         ├─ 存 PeerInfo
  │                         ├─ hello (B)  ─>
  ├─ 收到 B                 │
  ├─ 存 PeerInfo            │
  │                         │
  │  (现在双向 hello 完成)  │
  │                         │
  ├─ send_message("hi")     │
  │   ├─ store.insert       │
  │   ├─ pool.broadcast ────┼──> 收到 "hi"
  │   │                     │   ├─ store.insert (去重通过)
  │   │                     │   ├─ pool.broadcast (exclude=A)
  │   │                     │   └─ emit("message")
  │   └─ emit("message")    │
  │                         │
  ▼                         ▼
[前端收到 "hi"]          [前端收到 "hi"]
```

## 6. 错误与边界

| 场景 | 当前行为 |
|---|---|
| 重复 `node_id` 试图连接 | `PeerPool::add` 通过 DashMap key 去重，忽略第二次 |
| 收到超过 8000 字符的文本 | `Message::new` 截断到 8000 字符 |
| 同一消息循环回来 | `MessageStore::insert` 滑动窗口去重，丢弃 |
| 端口 4242-4252 全被占用 | `start_server` 返回 `Err`，应用启动失败 |
| mDNS 不可用 | 静默失败；用户可手动 `connect_peer` |
| WebSocket 断连 | 对端的写任务收到错误 → `PeerPool::remove` → 清理 |
