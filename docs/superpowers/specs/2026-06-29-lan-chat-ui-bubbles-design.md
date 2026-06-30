# lan-chat — UI 现代化（气泡 + 侧边栏 + 层次感）设计 spec

> 在保留「精炼版终端感」的前提下，引入现代 IM 的结构骨架：消息气泡、时间锚点、侧边栏、圆角边界、清晰分层。
> 编写时间：2026-06-29
> 状态：待用户 review

---

## 1. 背景与目标

**当前痛点**（`app.rs` 现状）：
- 消息是平铺 `horizontal_wrapped`，无独立容器、无气泡感
- 时间戳、device 标签、内容散落多行
- header / status bar 把信息都塞进一行 `horizontal`，没有分区
- 输入栏和状态栏之间没有「主操作 vs 配置入口」区分
- empty state 只有两行居中文字，缺引导

**目标**（与 2026-06-29-rust-native-ui-design spec 一致，不改大方向）：
- 保留 amber-on-black + JetBrains Mono 的终端基底识别度
- 引入 WhatsApp 风格的**结构骨架**（气泡、时间锚点、侧边栏）
- 引入**圆角与边界**（气泡 8px、输入框 8px、模态 12px）建立层次
- 把 manual connect / 改名等「配置」从主屏分离到侧边栏

**不做的事**：
- 不改色板（amber 主色 + green 点缀 + 深色克制）
- 不改字体（JetBrains Mono 唯一字体）
- 不改后端 / 协议 / 状态机
- 不引入新 crate

---

## 2. 视觉语言（既有 spec 之上的微调）

### 2.1 颜色（在 theme.rs 已有 token 上扩展）

新增 / 调整：

| Token | Hex | 用途 |
|---|---|---|
| `BUBBLE_SELF_BG` | `#1a1612` | 自己的消息气泡底（深暖色，比 BG 略亮） |
| `BUBBLE_OTHER_BG` | `#0f0f0e` | 他人消息气泡底（接近 BG） |
| `BUBBLE_SELF_BORDER` | `AMBER` (40% alpha) | 自己的气泡左边/右边线 |
| `BUBBLE_OTHER_BORDER` | `LINE` (`#1f201d`) | 他人的气泡边线 |
| `BUBBLE_CLIP_BG` | `#0d1a14` | 剪贴板消息气泡（带绿色调） |
| `BUBBLE_FILE_BG` | `#0d121a` | 文件消息气泡（带 amber 调） |
| `BUBBLE_SYS_BG` | `BG_ELEV` | 系统消息气泡（如连接/断开通知） |
| `RADIUS_SM` | `4.0` | 小元素（标签、徽章） |
| `RADIUS_MD` | `8.0` | 气泡、输入框、按钮 |
| `RADIUS_LG` | `12.0` | 模态、侧边栏 |
| `SHADOW` | 黑色 30% alpha, 2px offset, 4px blur | 仅用于模态/侧边栏浮层（不用于气泡） |

### 2.2 字号（不变）

`xs(12) / sm(13) / base(15) / lg(18) / xl(22)` — 沿用现有

### 2.3 间距（沿用 8px 网格 + 补 4px 微调）

`xxs(2) / xs(4) / sm(8) / md(12) / lg(16) / xl(24) / xxl(32)`

### 2.4 形状语言

- **气泡**：8px 圆角，左/右靠边贴齐（自己靠右，他人靠左）
- **输入框**：8px 圆角，1px `LINE` 边框，focused 时变 `AMBER`
- **模态/侧边栏**：12px 圆角，浮在 BG 之上，带轻微阴影
- **按钮**：8px 圆角，hairline 边框
- **徽章 / tag**：4px 圆角（如 peer 状态徽章）

### 2.5 动效（节制，保留终端感）

- 新消息入场：100ms `translateY(4px) → 0` + opacity 0→1
- 模态出现：150ms fade + 4px slide
- 闪烁块（caret）：保留现有 1.06s 闪烁节奏
- 不做：气泡 hover 抬升、输入框 focus 扩散圈、emoji 弹跳

---

## 3. 整体布局（侧边栏 + 主区）

```
┌────────────┬──────────────────────────────────────────┐
│            │ ▌ LAN-CHAT/0         ● 3 peers    [CLIP]│  ← Header (40px)
│            ├──────────────────────────────────────────┤
│  SIDEBAR   │                                          │
│            │   ── 10:30 ──                            │  ← 时间锚点
│  [设备名]  │                                          │
│            │      ┌────────────────────────┐          │
│  PEERS     │      │ hello from bob         │          │  ← 气泡（他人，靠左）
│  ● alice   │      └────────────────────────┘          │
│  ● bob     │                      ┌────────────────┐  │
│  ● charlie │                      │ hi alice!      │  │  ← 气泡（自己，靠右）
│            │                      └────────────────┘  │
│  CONFIGURE │                                          │
│  [改名字]  │      ┌────────────────────────┐          │
│  [连接]    │      │ 📎 design.png  234 KB  │          │  ← 文件气泡
│            │      │   [save]               │          │
│  STATUS    │      └────────────────────────┘          │
│  ip:port   │                                          │
│            ├──────────────────────────────────────────┤
│            │ [+ file]  type a message…          [↵]   │  ← 输入栏 (56px)
│            ├──────────────────────────────────────────┤
│            │ listening on 192.168.1.5:4242  ● online  │  ← Status (24px)
└────────────┴──────────────────────────────────────────┘
   200px           flex-1
```

**侧边栏**（左侧 200px，可点击切换显示/隐藏）：
- 顶部：本机名称（可点击改）
- 中部：peers 列表（带状态点 + 名字）
- 中部偏下：操作组
  - `[+ connect]` 按钮 — 弹 connect 模态或展开 IP:port 输入
  - `[rename]` 按钮 — 内联编辑
- 底部：ip:port / version 信息

**主区**（flex 1）：
- 顶部 header：品牌 + peer count + clip 按钮
- 中间 log：气泡列表
- 底部 input bar：极简
- 底部 status bar：极简

---

## 4. 组件原语（新增 `ui/components.rs`）

把所有 `RichText::new().font(...).color(...)` 拼装收敛到组件工厂：

```rust
// 标签
pub fn label_mono(text, size: FontSize, color: Color32) -> impl Widget
pub fn label_meta(text) // muted + xs
pub fn label_section(text) // amber + sm + uppercase

// 按钮
pub fn button_ghost(label) // hairline border
pub fn button_primary(label) // amber fill, dark text
pub fn button_icon(icon_text) // [+], [↵], [×] 之类
pub fn button_toggle(pressed, label) // 侧边栏的 [connect] / [rename]

// 卡片 / 容器
pub fn card_bubble_self(content_fn) // amber border-left
pub fn card_bubble_other(content_fn) // line border
pub fn card_section(content_fn) // 大块容器

// 徽章
pub fn badge_status(online: bool) // ● / ○
pub fn badge_device(name, is_self) // [name] / [name (you)]

// 锚点
pub fn time_anchor(ts) // ── 10:30 ── 居中

// 输入
pub fn input_field(text, hint, width) // 8px 圆角 input
```

所有这些函数**只返回 widget 配置，不持有状态**。状态仍在 `LanChatApp` 上。

---

## 5. 消息气泡详细规范

### 5.1 三种消息类型 × 视觉

| 类型 | 位置 | 背景 | 左边线 | 文字色 | 内部结构 |
|---|---|---|---|---|---|
| 自己 text | 右对齐 | `BUBBLE_SELF_BG` | `AMBER` 4px | `TEXT` | 单行：内容；顶行小字：time + "you" |
| 他人 text | 左对齐 | `BUBBLE_OTHER_BG` | `LINE` 1px | `TEXT` | 顶行小字：time + device；主体：内容 |
| 自己 clipboard | 右对齐 | `BUBBLE_CLIP_BG` | `GREEN` 4px | `GREEN` 弱化 | 顶行：[CLIPBOARD] + time + you；主体：内容 |
| 他人 clipboard | 左对齐 | `BUBBLE_CLIP_BG` | `LINE` 1px | `GREEN` | 顶行：[CLIPBOARD] + time + device；主体：内容 |
| 自己 file | 右对齐 | `BUBBLE_FILE_BG` | `AMBER` 4px | `TEXT` | 顶行：📎 + filename + size + time；主体：[open] / [save] 按钮 |
| 他人 file | 左对齐 | `BUBBLE_FILE_BG` | `LINE` 1px | `TEXT` | 顶行：📎 + filename + size + device + time；主体：[save] 按钮 |

### 5.2 气泡内部布局

```
┌─ 顶行（meta，xs 字号，muted 色）──────────────────┐
│ 14:32 ● bob                                       │
├─ 主体（base 字号，正文色）──────────────────────┤
│ the message text wraps here as long as needed    │
│ second line if any                               │
└─ 底行（可选：状态徽章）─────────────────────────┘
```

- 顶行高 16px，主体 padding 8px
- 气泡外边距：自己右对齐时右侧 0、左侧 32px；他人左对齐时左侧 0、右侧 32px
- 气泡最大宽度：可用宽度的 70%
- 气泡 padding：8px 12px
- 气泡 corner radius：8px

### 5.3 时间锚点

每 30 分钟或跨日时插入一个居中锚点：

```
────────── 14:30 ──────────
```

- 1px `LINE` 水平线，中间是时间字符串
- muted 颜色、xs 字号
- 仅在有消息时显示，empty state 不显示

### 5.4 连续消息聚合

同一 device 在 60s 内的连续消息**合并到一个气泡组**：
- 第一个气泡顶部显示 device + time
- 后续气泡只显示内容、time 隐藏
- 视觉上像「一个人连发了几条」

---

## 6. 各区域详细规范

### 6.1 Header（顶部 40px）

```
█ LAN-CHAT/0    ● 3 peers                       [ CLIP ]
```

- 左：amber 块（保留） + `LAN-CHAT/0` 标题
- 中：peer 状态徽章 `● N peers` / `● online`（绿点 + 文字）
- 右：`[ CLIP ]` 按钮（ghost 样式）
- 整行 hairline 下边框

### 6.2 Sidebar（左侧 200px，可折叠）

```
┌────────────────────┐
│  THIS TERMINAL     │  ← section label
│  ● alice           │  ← self badge
│                    │
│  PEERS  [3]        │  ← section label
│  ● bob             │  ← status badge + name
│  ● charlie         │
│  ○ david (offline) │
│                    │
│  CONFIGURE         │  ← section label
│  [ rename ]        │  ← toggle button
│  [ connect ]       │  ← toggle button (展开 IP:port 输入)
│                    │
│  ─────────────     │  ← divider
│  192.168.1.5:4242  │  ← meta
│  v0.1.0            │
└────────────────────┘
```

- 背景：`BG_ELEV`
- 右侧 1px `LINE` 边界
- 顶部 `[×]` 按钮折叠（折叠后只剩 32px 宽的侧边栏 icon strip）
- peer 项：可点击（未来：点击看设备详情）

### 6.3 Input Bar（底部 56px）

```
┌──────────────────────────────────────────────────────────┐
│ [+] │ type a message…                          │  ↵  │
└──────────────────────────────────────────────────────────┘
```

- 单行水平布局
- `[+]` 文件选择按钮（amber，icon 风格）
- `│` 分隔线 1px `LINE`
- 中间 TextEdit：8px 圆角，1px `LINE` 边框，focused 时 `AMBER`
- 右侧 `↵` 发送按钮：空输入时 muted，否则 amber
- 高度固定 56px，padding 8px 16px

### 6.4 Status Bar（底部 24px）

```
listening on 192.168.1.5:4242      ● online      v0.1.0
```

- 单行，meta 字号（xs）
- 极简：左 = listening 地址 / 中 = 连接状态 / 右 = 版本号
- `BG_ELEV` 背景，hairline 上边框
- **改名 / connect 入口已移到侧边栏**

### 6.5 Empty State

```
                  ▌
                  
           no peers on the wire.
           
       your terminal: 192.168.1.5:4242
       
     [+] invite by IP:port    or    wait for mDNS
```

- 居中布局
- 大号 amber 闪烁块（保留识别度）
- 主文案（base）+ meta 提示（sm muted）
- 两个引导按钮：手动 connect / 等待 mDNS

### 6.6 Clip Modal

- 居中模态，背景半透明黑 + 4px blur
- 模态本体：12px 圆角，`BG_ELEV` 填充，1px `LINE` 边框
- 标题：`[ CLIPBOARD SHARE ]`
- 预填 arboard 读取的剪贴板内容
- 操作：`[ send ]` 主按钮 / `[ cancel ]` ghost

### 6.7 File Card（嵌入气泡内）

```
┌─────────────────────────────────────┐
│ 📎 design.png          234 KB      │
│ sha: a1b2c3…                       │
│ [ save to disk ]   [ open ]        │
└─────────────────────────────────────┘
```

- 在文件气泡内
- 顶行：filename + size
- 中行：sha256 短码
- 底行：操作按钮
- 已下载：状态徽章 `✓ saved`

---

## 7. 文件结构（最终）

```
src-tauri/src/
├── app.rs                 # eframe::App 主循环（瘦下来）
├── ui/
│   ├── theme.rs           # 扩展：新增 BUBBLE_*/RADIUS_*/SHADOW token
│   ├── components.rs      # 新增：组件原语（labels/buttons/cards/badges）
│   ├── bubble.rs          # 新增：消息气泡渲染（text/clipboard/file）
│   ├── sidebar.rs         # 新增：侧边栏（peers + configure）
│   ├── header.rs          # 抽离自 app.rs
│   ├── input_bar.rs       # 抽离自 app.rs
│   ├── status_bar.rs      # 抽离自 app.rs
│   ├── empty_state.rs     # 抽离自 app.rs
│   ├── setup.rs           # （保留，可能微调）
│   ├── clip_modal.rs      # 重写视觉，逻辑不变
│   ├── file_card.rs       # 重写视觉，逻辑不变
│   └── mod.rs             # 导出
```

`app.rs` 最终只负责：状态机 + event drain + 调度各 UI 区域。

---

## 8. 实施 Phase（建议一次性完成）

1. **theme 扩展 + components.rs** — 加 token，加组件工厂
2. **bubble.rs** — 气泡组件 + 三种消息类型 + 时间锚点 + 连续消息聚合
3. **sidebar.rs + header/input_bar/status_bar/empty_state 抽离** — layout 重排
4. **clip_modal + file_card 视觉重写** — 收敛到新组件语言

每 phase 内部一次 commit，全部 4 phase 一次跑通再 review。

---

## 9. 风险与权衡

| 风险 | 缓解 |
|---|---|
| 改圆角与原 spec 「no rounded bubble」冲突 | 用户已明确要圆角，本次 spec 是对原 spec 的 update |
| 侧边栏占用主区宽度 | 200px 是 desktop IM 的标准做法；可折叠 |
| mono 字体 + 圆角 + 浅色 = 风格不统一 | 仍保留深色背景 + amber 主色，mono 是统一锚点 |
| 实施量大 | 拆 4 phase 但一次跑通；预估 600-800 行新代码 |
| 动效过多破坏终端克制感 | 严控：仅入场 fade + caret blink；不做 hover 抬升 |

---

## 10. 完成定义

- [ ] `cargo run` 启动后看到：侧边栏 + 主区（header/气泡流/输入栏/状态栏）
- [ ] 自己消息：右对齐、amber 边线、BUBBLE_SELF_BG 底
- [ ] 他人消息：左对齐、LINE 边线、BUBBLE_OTHER_BG 底
- [ ] 文件消息：独立文件气泡
- [ ] 剪贴板消息：绿色边线标识
- [ ] 30 分钟 / 跨日时间锚点
- [ ] 同一 device 60s 内连续消息合并
- [ ] 侧边栏可折叠
- [ ] 输入栏 / 模态 / 气泡全部 8px 圆角
- [ ] 所有视觉 token 走 theme.rs，app.rs 不出现裸 hex
- [ ] macOS + Ubuntu 各跑一次手动验证
