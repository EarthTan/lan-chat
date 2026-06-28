const express = require('express');
const http    = require('http');
const { Server } = require('socket.io');
const path    = require('path');
const os      = require('os');

const app    = express();
const server = http.createServer(app);
const io     = new Server(server);

app.use(express.static(path.join(__dirname, 'public')));

// In-memory ring buffer — no disk writes, no external DB
const messages = [];
const MAX = 200;

io.on('connection', (socket) => {
  socket.emit('history', messages);

  socket.on('message', (data) => {
    const msg = {
      id:     `${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
      text:   String(data.text  || '').trim().slice(0, 8000),
      device: String(data.device || 'Unknown').trim().slice(0, 40),
      type:   data.type === 'clipboard' ? 'clipboard' : 'text',
      ts:     Date.now(),
    };
    if (!msg.text) return;
    messages.push(msg);
    if (messages.length > MAX) messages.shift();
    io.emit('message', msg);
  });
});

const PORT = process.env.PORT || 4242;
server.listen(PORT, '0.0.0.0', () => {
  console.log('\n🚀  LAN Chat is running!\n');
  Object.values(os.networkInterfaces())
    .flat()
    .filter(n => n.family === 'IPv4' && !n.internal)
    .forEach(n => console.log(`   → http://${n.address}:${PORT}`));
  console.log('\n   Ctrl+C to stop.\n');
});
