# LAN Chat

纯局域网即时通讯 + 剪贴板共享，无外部服务器，无账号，无云端。

## 功能

- 💬 实时聊天（WebSocket，低延迟）
- 📋 剪贴板内容一键分享
- 🌑 现代深色 UI，移动端适配
- 💾 内存消息缓存（最近 200 条，重启清空，保护隐私）
- 📡 纯局域网，流量不出本地网络

---

## 快速启动

**要求：** Node.js 18+

```bash
cd lan-chat
npm install        # 仅首次需要
npm start
```

终端会打印本机 LAN IP：

```
🚀  LAN Chat is running!

   → http://192.168.1.42:4242
```

在 Mac 和 Ubuntu 各自用浏览器打开该地址即可。  
首次打开时输入设备名（存 localStorage，之后免输入）。

---

## 剪贴板使用说明

### 发送端
1. 在源设备复制内容（⌘C / Ctrl+C）
2. 浏览器中点击右上角 **「分享剪贴板」**
3. 如果是 **localhost** 或 **HTTPS** 访问，内容会自动填入；  
   如果是 LAN IP 访问（http://），浏览器出于安全策略拒绝自动读取，**手动粘贴**到弹窗即可（⌘V / Ctrl+V）

### 接收端
- 悬停消息气泡 → 点击「**复制**」按钮 → 写入本机剪贴板

> 📝 发送代码片段、URL、命令等长文本时，剪贴板气泡（紫色）和普通消息都很好用。

---

## 设为 Ubuntu 开机自启（systemd）

```bash
sudo nano /etc/systemd/system/lan-chat.service
```

写入以下内容（根据实际路径修改）：

```ini
[Unit]
Description=LAN Chat
After=network.target

[Service]
Type=simple
User=asyncb
WorkingDirectory=/home/asyncb/lan-chat
ExecStart=/usr/bin/node server.js
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable lan-chat
sudo systemctl start lan-chat

# 查看状态
sudo systemctl status lan-chat
```

---

## 可选：HTTPS 支持（解锁自动读取剪贴板）

用 mkcert 生成本地受信任证书：

```bash
# Ubuntu
sudo apt install mkcert
mkcert -install
mkcert 192.168.1.42   # 替换为实际 IP

# 在 server.js 启用 HTTPS（替换 http.createServer）：
# const https = require('https');
# const fs = require('fs');
# const server = https.createServer({
#   key:  fs.readFileSync('192.168.1.42-key.pem'),
#   cert: fs.readFileSync('192.168.1.42.pem'),
# }, app);
```

启用后，Mac 也需安装 mkcert 根证书（`mkcert -install`），  
浏览器访问 `https://192.168.1.42:4242` 即可自动读写剪贴板。

---

## 端口修改

```bash
PORT=8080 npm start
```
