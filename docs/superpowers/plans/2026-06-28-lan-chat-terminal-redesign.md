# LAN Chat — TERMINAL/0 Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite `public/index.html` into a terminal-styled, JetBrains-Mono, amber-on-black LAN chat UI that preserves all current behavior.

**Architecture:** Single static HTML file. All styles live in one `<style>` block driven by CSS custom properties (the six design tokens). Markup is restructured into setup overlay → header → log → input → modal → toast, all flat siblings of `<body>`. JS keeps the existing socket flow; only the render function and event handlers change where required by the new markup.

**Tech Stack:** HTML5, vanilla CSS (custom properties + keyframes), vanilla JS (no framework), Socket.IO client (existing), Google Fonts (JetBrains Mono). Tailwind CDN is removed.

## Global Constraints

From the spec — these apply to every task unless overridden:

- Colors only from the palette: `#0A0A0A`, `#111211`, `#1F201D`, `#FFB000`, `#7CFFB2`, `#D9D6CC`, `#5C5D54`.
- Single font family: JetBrains Mono, with fallbacks `Menlo, Consolas, "Courier New", monospace`, and CJK fallback `"PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", system-ui, sans-serif`.
- Type scale: `11 / 12 / 13 / 15 / 28` px. Weights: 400 / 500 / 700. No italic anywhere.
- No rounded "bubble" geometry on messages. Borders are 1 px hairlines in `#1F201D`. No shadows except the single soft shadow on the setup card.
- Block caret keyframe is the only recurring decoration: `1.06s steps(2) infinite` between `opacity:1` and `opacity:0`.
- All touch targets ≥ 44 px high.
- `@media (prefers-reduced-motion: reduce)` must freeze every animation and transition.
- No emoji on the page except the single `📋` in the clipboard modal title.
- No new dependencies beyond the JetBrains Mono Google Fonts link.
- No server-side changes; `server.js`, `package.json`, `README.md` are untouched.
- localStorage key `lc-name` continues to store the device name.
- WebSocket protocol unchanged.

---

## File Structure

- **Modify:** `public/index.html` (only file touched)
  - `<head>` — Google Fonts preconnect + link, single `<style>` block with tokens + base + component rules + keyframes + reduced-motion
  - `<body>` — setup overlay, header, log region, input bar, clipboard modal, toast
  - `<script>` — preserved socket flow, rewritten `render()`, added boot-sequence driver, added `[..]` indicator animator

---

## Task Index

- Task 1: Foundation — head, fonts, tokens, base reset
- Task 2: Setup overlay — boot sequence card with timed `[OK]` reveal and `[..]` animator
- Task 3: Header — brand row, online dot, device badge, `[ CLIP ]` button
- Task 4: Log region — empty state with breathing block, scroll container
- Task 5: Message render — log-row anatomy with timestamp, device, hairline, content, hover meta
- Task 6: Input bar — prompt composition, blinking caret, hairline-underline focus state
- Task 7: Clipboard modal — backdrop, dashed-amber textarea, `[ SEND ↵ ]` CTA
- Task 8: Toast — single-row, fade in/hold/fade out
- Task 9: Reduced-motion + final accessibility sweep
- Task 10: Smoke verify — visual + behavioral checks against success criteria

---

### Task 1: Foundation — head, fonts, tokens, base reset

**Files:**
- Modify: `public/index.html` (replace lines 1–13 — the entire `<head>`)

**Interfaces:**
- Produces: CSS custom properties `--bg`, `--bg-elev`, `--line`, `--amber`, `--green`, `--text`, `--muted` on `:root`
- Produces: keyframes `blink`, `breathe`, `rowIn`, `shake`
- Produces: `.caret` class for the 12×18 px amber blinking block

- [ ] **Step 1: Replace `<head>` with the new foundation**

Replace from line 1 through line 37 (the closing `</style>` of the existing head) with:

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
  <meta name="color-scheme" content="dark">
  <title>LAN-CHAT/0</title>

  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link rel="stylesheet"
        href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500;700&display=swap">

  <script src="/socket.io/socket.io.js"></script>

  <style>
    :root {
      --bg:      #0A0A0A;
      --bg-elev: #111211;
      --line:    #1F201D;
      --amber:   #FFB000;
      --green:   #7CFFB2;
      --text:    #D9D6CC;
      --muted:   #5C5D54;

      --mono: "JetBrains Mono", Menlo, Consolas, "Courier New", monospace,
              "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", system-ui, sans-serif;
    }

    *, *::before, *::after { box-sizing: border-box; }
    html, body { height: 100%; margin: 0; overflow: hidden; }
    body {
      background: var(--bg);
      color: var(--text);
      font-family: var(--mono);
      font-size: 13px;
      line-height: 1.55;
      -webkit-font-smoothing: antialiased;
      -moz-osx-font-smoothing: grayscale;
    }

    button {
      font: inherit;
      color: inherit;
      background: none;
      border: none;
      padding: 0;
      cursor: pointer;
    }
    input, textarea {
      font: inherit;
      color: inherit;
      background: none;
      border: none;
      outline: none;
    }

    :focus-visible {
      outline: 1px solid var(--amber);
      outline-offset: 2px;
    }

    /* ── Keyframes ─────────────────────────────────────────── */
    @keyframes blink {
      0%, 49.999% { opacity: 1; }
      50%, 100%   { opacity: 0; }
    }
    @keyframes breathe {
      0%, 100% { opacity: 0.35; }
      50%      { opacity: 1; }
    }
    @keyframes rowIn {
      from { opacity: 0; transform: translateY(4px); }
      to   { opacity: 1; transform: translateY(0); }
    }
    @keyframes shake {
      0%, 100% { transform: translateX(0); }
      25%      { transform: translateX(-3px); }
      50%      { transform: translateX(3px); }
      75%      { transform: translateX(-2px); }
    }

    /* ── The signature caret ───────────────────────────────── */
    .caret {
      display: inline-block;
      width: 12px;
      height: 18px;
      background: var(--amber);
      vertical-align: text-bottom;
      animation: blink 1.06s steps(2) infinite;
    }
    .caret--breathe {
      animation: breathe 2.4s ease-in-out infinite;
    }

    @media (prefers-reduced-motion: reduce) {
      *, *::before, *::after {
        animation-duration: 0.001ms !important;
        animation-iteration-count: 1 !important;
        transition-duration: 0.001ms !important;
      }
      .caret, .caret--breathe { animation: none; opacity: 0.85; }
    }
  </style>
</head>
```

- [ ] **Step 2: Save and confirm structure**

Save the file. Open it; confirm `<head>` ends with `</head>` and the body tag on line 38 still says `<body>`. The `<script src="/socket.io/socket.io.js"></script>` line must still be present in the head.

- [ ] **Step 3: Commit**

The repo is not a git repository (verified earlier with `git status` → "fatal: 不是 Git 仓库"). Skip the commit step and note in the plan log: "Repo not under git; manual snapshot taken before Task 1."

**Verification for this task:** Open the file in a browser; page should render with black background, JetBrains Mono loaded. No visible caret yet (added in Task 2/4/6).

---

### Task 2: Setup overlay — boot sequence card

**Files:**
- Modify: `public/index.html` lines 39–64 (the current setup overlay block)

**Interfaces:**
- Consumes: `.caret` from Task 1
- Consumes: boot config — three status lines, each with a state (`pending`, `ok`, `progressing`)
- Produces: `id="setup"`, `id="boot1"`, `id="boot2"`, `id="boot3"`, `id="nameIn"`, `id="startBtn"`
- Produces: `saveName()` updates — adds shake on empty submit, still calls same flow

- [ ] **Step 1: Replace the setup markup (lines 39–64)**

```html
<body>

  <!-- ── Setup overlay ── -->
  <div id="setup" class="fixed inset-0" style="z-index:50;background:var(--bg);display:flex;align-items:center;justify-content:center;padding:24px;">
    <div class="setup-card">
      <div class="setup-head">
        <span>LAN-CHAT/0</span>
        <span class="setup-head__rule"></span>
        <span class="setup-head__ver">v1.0.0</span>
      </div>

      <div class="setup-boot">
        <div class="boot-line" id="boot1">
          <span class="boot-msg">initializing p2p socket</span>
          <span class="boot-dots"><span>.</span><span>.</span><span>.</span></span>
          <span class="boot-tag" data-state="pending">[ .. ]</span>
        </div>
        <div class="boot-line" id="boot2">
          <span class="boot-msg">binding 0.0.0.0:4242</span>
          <span class="boot-dots"><span>.</span><span>.</span><span>.</span></span>
          <span class="boot-tag" data-state="pending">[ .. ]</span>
        </div>
        <div class="boot-line" id="boot3">
          <span class="boot-msg">awaiting peer handshake</span>
          <span class="boot-dots"><span>.</span><span>.</span><span>.</span></span>
          <span class="boot-tag" data-state="pending">[ .. ]</span>
        </div>
      </div>

      <label class="setup-label" for="nameIn">identify this terminal:</label>
      <div class="setup-input">
        <span class="setup-prompt">›</span>
        <input id="nameIn" type="text" maxlength="24" autocomplete="off" spellcheck="false">
        <span class="caret" id="nameCaret"></span>
      </div>

      <div class="setup-foot">
        <span class="setup-hint">enter accepts · esc dismisses overlay</span>
        <div class="setup-actions">
          <button id="startBtn" type="button" class="btn-primary">[ ENTER ↵ ]   connect</button>
        </div>
      </div>
    </div>
  </div>
```

- [ ] **Step 2: Add setup-specific styles (append inside the existing `<style>` block, before `</style>`)**

```css
    /* ── Setup overlay ─────────────────────────────────────── */
    .setup-card {
      width: 100%;
      max-width: 460px;
      background: var(--bg-elev);
      border: 1px solid var(--line);
      padding: 28px 28px 22px;
      box-shadow: 0 24px 60px rgba(0, 0, 0, 0.6);
      display: flex;
      flex-direction: column;
      gap: 18px;
    }
    .setup-head {
      display: flex;
      align-items: center;
      gap: 10px;
      font-size: 13px;
      font-weight: 500;
      color: var(--text);
    }
    .setup-head__rule {
      flex: 1;
      height: 1px;
      background: var(--line);
    }
    .setup-head__ver {
      font-size: 11px;
      color: var(--muted);
    }
    .setup-boot {
      display: flex;
      flex-direction: column;
      gap: 4px;
      font-size: 12px;
      color: var(--muted);
    }
    .boot-line {
      display: grid;
      grid-template-columns: 1fr auto auto;
      align-items: center;
      gap: 10px;
    }
    .boot-dots {
      display: inline-flex;
      gap: 2px;
    }
    .boot-dots span {
      animation: dots 1.2s steps(3) infinite;
      opacity: 0.25;
    }
    .boot-dots span:nth-child(2) { animation-delay: 0.2s; }
    .boot-dots span:nth-child(3) { animation-delay: 0.4s; }
    @keyframes dots {
      0%, 60%, 100% { opacity: 0.25; }
      30%           { opacity: 1; }
    }
    .boot-tag {
      font-size: 11px;
      letter-spacing: 0.02em;
    }
    .boot-tag[data-state="ok"]   { color: var(--green); }
    .boot-tag[data-state="wait"] { color: var(--amber); }

    .setup-label {
      font-size: 11px;
      color: var(--muted);
      text-transform: lowercase;
    }
    .setup-input {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 0;
      border-bottom: 1px solid var(--line);
      min-height: 44px;
    }
    .setup-input:focus-within { border-bottom-color: var(--amber); }
    .setup-prompt { color: var(--amber); font-weight: 500; }
    .setup-input input {
      flex: 1;
      min-width: 0;
      font-size: 15px;
      color: var(--text);
      caret-color: transparent; /* we draw our own caret */
    }
    .setup-input.shake { animation: shake 120ms ease-in-out; }

    .setup-foot {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 16px;
      flex-wrap: wrap;
    }
    .setup-hint {
      font-size: 11px;
      color: var(--muted);
    }
    .btn-primary {
      background: var(--amber);
      color: var(--bg);
      font-weight: 700;
      font-size: 12px;
      padding: 10px 14px;
      min-height: 44px;
      transition: filter 120ms ease-out;
    }
    .btn-primary:hover  { filter: brightness(1.1); }
    .btn-primary:active { transform: translateY(1px); }

    @media (prefers-reduced-motion: reduce) {
      .boot-dots span { animation: none; opacity: 0.6; }
    }
```

- [ ] **Step 3: Replace setup-related JS (the two `DOMContentLoaded` blocks + `saveName`)**

Replace lines 175–207 with:

```js
  // ── Init ─────────────────────────────────────────────────
  addEventListener('DOMContentLoaded', () => {
    const saved = localStorage.getItem('lc-name');
    if (saved) {
      ME = saved;
      document.getElementById('myBadge').textContent = ME;
      document.getElementById('setup').style.display = 'none';
    } else {
      const ua = navigator.userAgent;
      let hint = '我的设备';
      if (/Mac OS X/.test(ua) && !/iPhone|iPad/.test(ua)) hint = 'Alex 的 Mac';
      else if (/Linux/.test(ua))   hint = 'Ubuntu';
      else if (/Windows/.test(ua)) hint = '我的 PC';
      const inp = document.getElementById('nameIn');
      inp.value = hint;
      inp.focus();
      inp.setSelectionRange(hint.length, hint.length);

      // ── Boot sequence ────────────────────────────────────
      const tagFor = id => document.querySelector(`#${id} .boot-tag`);
      const dotsFor = id => document.querySelectorAll(`#${id} .boot-dots span`);
      const setOk = id => {
        tagFor(id).textContent = '[ OK ]';
        tagFor(id).dataset.state = 'ok';
        dotsFor(id).forEach(d => d.style.animation = 'none');
      };
      setTimeout(() => setOk('boot1'), 120);
      setTimeout(() => setOk('boot2'), 240);
      // boot3 stays [..] until connect
      tagFor('boot3').dataset.state = 'wait';
    }
  });

  function saveName() {
    const inp  = document.getElementById('nameIn');
    const wrap = inp.parentElement;
    const v    = inp.value.trim();
    if (!v) {
      wrap.classList.remove('shake');
      void wrap.offsetWidth;       // restart animation
      wrap.classList.add('shake');
      inp.focus();
      return;
    }
    ME = v;
    localStorage.setItem('lc-name', ME);
    document.getElementById('myBadge').textContent = ME;
    const s = document.getElementById('setup');
    s.style.transition = 'opacity 160ms ease-out';
    s.style.opacity = '0';
    setTimeout(() => s.style.display = 'none', 180);
    document.getElementById('inp').focus();
  }

  document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('startBtn').addEventListener('click', saveName);
    document.getElementById('nameIn').addEventListener('keydown', e => {
      if (e.key === 'Enter') saveName();
    });
  });
```

- [ ] **Step 4: Verify visually**

Save and open in a browser (clear `localStorage` first if needed). Confirm:

- Three boot lines visible, first two switch to `[ OK ]` in green within ~300 ms.
- Third line stays `[ .. ]` in amber until "ENTER" pressed.
- Input field underlined by hairline; pressing Enter with empty input triggers a shake; valid input fades overlay out in 160 ms.

---

### Task 3: Header — brand row, online dot, badge, `[ CLIP ]`

**Files:**
- Modify: `public/index.html` (replace the existing `<header>` block — currently lines 66–90)

**Interfaces:**
- Consumes: `.caret` from Task 1
- Produces: `id="header"`, `id="myBadge"`, `id="clipBtn"`
- The `openClip()` function is defined later in Task 7 — make sure the click handler is wired here but the function may not exist yet; gate it with `if (typeof openClip === 'function') openClip();` until Task 7 lands.

- [ ] **Step 1: Replace the header markup (current lines 66–90)**

```html
  <!-- ── Header ── -->
  <header id="header" class="app-header">
    <div class="app-header__left">
      <span class="caret" aria-hidden="true"></span>
      <span class="brand">LAN-CHAT/0</span>
      <span class="online" aria-live="polite">
        <span class="online__dot" aria-hidden="true"></span>
        <span class="online__label">online</span>
      </span>
    </div>
    <div class="app-header__right">
      <span id="myBadge" class="device-badge">[anonymous]</span>
      <button id="clipBtn" type="button" class="clip-btn">[ CLIP ]</button>
    </div>
  </header>
```

- [ ] **Step 2: Add header styles (append inside `<style>`)**

```css
    /* ── Header ───────────────────────────────────────────── */
    .app-header {
      height: 40px;
      flex: 0 0 auto;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 0 16px;
      background: var(--bg-elev);
      border-bottom: 1px solid var(--line);
    }
    .app-header__left,
    .app-header__right {
      display: flex;
      align-items: center;
      gap: 10px;
      min-width: 0;
    }
    .brand {
      font-size: 13px;
      font-weight: 500;
      color: var(--text);
    }
    .online {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-size: 11px;
      color: var(--muted);
    }
    .online__dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--green);
      box-shadow: 0 0 0 2px rgba(124, 255, 178, 0.15);
    }
    .device-badge {
      font-size: 11px;
      color: var(--muted);
      border: 1px solid var(--line);
      padding: 4px 8px;
      max-width: 140px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .clip-btn {
      font-size: 12px;
      font-weight: 500;
      color: var(--text);
      border: 1px solid var(--line);
      padding: 0 12px;
      min-height: 44px;
      transition: color 120ms ease-out, border-color 120ms ease-out;
    }
    .clip-btn:hover {
      color: var(--amber);
      border-color: var(--amber);
    }
    @media (max-width: 420px) {
      .online__label,
      .device-badge { display: none; }
      .clip-btn::before { content: '[ · ]'; }
      .clip-btn { font-size: 0; padding: 0 10px; }
      .clip-btn:hover::before { color: var(--amber); }
    }
```

- [ ] **Step 3: Wire the header badge defaults**

In the `<script>` block, locate the existing block:

```js
  document.getElementById('myBadge').textContent = ME;
```

(it appears in both `DOMContentLoaded` saved branch and `saveName`). Keep both. No other JS changes for this task.

- [ ] **Step 4: Verify**

Reload. Header should show `[▌] LAN-CHAT/0 ● online   [anonymous]  [ CLIP ]` with everything left-aligned and `online`/`badge` hidden under 420 px wide.

---

### Task 4: Log region — empty state

**Files:**
- Modify: `public/index.html` (replace the `<main id="msgs">` block — currently lines 92–102)

**Interfaces:**
- Consumes: `.caret--breathe` from Task 1
- Produces: `<main id="msgs">` with an `id="empty"` block
- JS in Task 5 removes `empty` on first message; nothing in this task touches it

- [ ] **Step 1: Replace the log region markup**

```html
  <!-- ── Log region ── -->
  <main id="msgs" class="log" aria-live="polite" aria-label="message log">
    <div id="empty" class="log-empty">
      <p class="log-empty__line">no peers on the wire.</p>
      <p class="log-empty__line log-empty__host">listening on <span id="emptyHost">—</span></p>
      <span class="caret caret--breathe log-empty__block" aria-hidden="true"></span>
    </div>
  </main>
```

- [ ] **Step 2: Add log styles**

```css
    /* ── Log region ───────────────────────────────────────── */
    .log {
      flex: 1 1 auto;
      min-height: 0;
      overflow-y: auto;
      overflow-x: hidden;
      padding: 16px 20px 24px;
      scroll-behavior: smooth;
    }
    .log::-webkit-scrollbar         { width: 4px; }
    .log::-webkit-scrollbar-thumb   { background: var(--line); }
    .log::-webkit-scrollbar-track   { background: transparent; }

    .log-empty {
      min-height: 100%;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 6px;
      text-align: center;
      color: var(--muted);
      pointer-events: none;
      user-select: none;
    }
    .log-empty__line {
      margin: 0;
      font-size: 13px;
    }
    .log-empty__host {
      font-size: 12px;
    }
    .log-empty__block {
      margin-top: 28px;
      width: 64px;
      height: 64px;
    }
```

- [ ] **Step 3: Set the host label**

Add to the script block, immediately after `const socket = io();`:

```js
  // Reflect LAN/local host for the empty state.
  addEventListener('DOMContentLoaded', () => {
    const h = document.getElementById('emptyHost');
    if (h) h.textContent = window.location.host || 'localhost';
  });
```

- [ ] **Step 4: Verify**

Reload on an empty server (no messages). Should show two muted lines with the breathing amber block. Resize browser < 420 px: header collapses; log area still readable.

---

### Task 5: Message render — log-row anatomy

**Files:**
- Modify: `public/index.html` (replace the `render()` function — currently lines 270–316)

**Interfaces:**
- Consumes: `#msgs` and `#empty` from Task 4
- Consumes: `txtById` map (kept for clipboard copy in Task 7)
- Produces: row markup `.row` with `[HH:MM:SS]` / device / hairline / content / hover-meta strip
- Produces: `.row--me` (amber content), `.row--them` (text color), `.row--clip` (green + `[clip]` chip)
- Produces: row carries `data-meta` attribute (`id 8a3f · 42 B`) shown on `:hover` via CSS
- Removes the `#empty` element on first render

- [ ] **Step 1: Replace `render()`**

```js
  // ── Render message ────────────────────────────────────────
  function render(msg, animate) {
    txtById[msg.id] = msg.text;

    const emEl = document.getElementById('empty');
    if (emEl) emEl.remove();

    const isMe   = msg.device === ME;
    const isClip = msg.type === 'clipboard';
    const time   = new Date(msg.ts).toLocaleTimeString('zh-CN',
                      { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
    const esc    = s => String(s)
      .replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')
      .replace(/"/g,'&quot;').replace(/'/g,'&#39;');

    const safeId   = esc(msg.id);
    const shortId  = safeId.slice(-4);
    const bytes    = new Blob([msg.text]).size;
    const meta     = `id ${shortId} · ${bytes} B`;

    const role = isClip ? 'row--clip' : isMe ? 'row--me' : 'row--them';

    const row = document.createElement('article');
    row.className = `row ${role}${animate ? ' row--in' : ''}`;
    row.dataset.meta = meta;

    const clipChip = isClip
      ? `<span class="row__chip">[clip]</span>`
      : '';

    row.innerHTML = `
      <header class="row__head">
        <span class="row__time">[${esc(time)}]</span>
        <span class="row__device">${esc(msg.device)}</span>
        ${clipChip}
        <span class="row__rule" aria-hidden="true"></span>
      </header>
      <div class="row__body">${esc(msg.text)}</div>
      <div class="row__meta">${meta}</div>
    `;
    document.getElementById('msgs').appendChild(row);
  }
```

- [ ] **Step 2: Add row styles**

```css
    /* ── Message rows ─────────────────────────────────────── */
    .row {
      display: block;
      padding: 6px 0 8px;
      border-bottom: 0;
    }
    .row + .row { margin-top: 0; }
    .row--in { animation: rowIn 100ms ease-out both; }

    .row__head {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 11px;
      color: var(--muted);
      font-weight: 500;
    }
    .row__time   { letter-spacing: 0.02em; }
    .row__device { color: var(--muted); }
    .row__rule {
      flex: 1 1 auto;
      height: 1px;
      background: var(--line);
      min-width: 12px;
    }
    .row__chip {
      font-size: 10px;
      color: var(--green);
      border: 1px solid var(--green);
      padding: 0 5px;
      line-height: 14px;
    }

    .row__body {
      font-size: 13px;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-word;
      margin: 4px 0 0 12px;
      color: var(--text);
      user-select: text;
    }
    .row--me   .row__body { color: var(--amber); }
    .row--clip .row__body {
      color: var(--green);
      font-feature-settings: "calt" 0, "liga" 0;
    }

    .row__meta {
      margin: 4px 0 0 12px;
      font-size: 10px;
      color: var(--muted);
      opacity: 0;
      transition: opacity 120ms ease-out;
    }
    .row:hover .row__meta { opacity: 1; }
```

- [ ] **Step 3: Verify**

Send two messages from two devices (or use `socket.emit('message', ...)` in devtools). Confirm:

- Each row has timestamp / device / hairline / body.
- Own row's body is amber; peer's is off-white; clipboard rows are green with `[clip]` chip.
- Hovering shows `id xxxx · NN B` after 120 ms.
- New rows fade-in over 100 ms.
- Empty-state block disappears on first message.

---

### Task 6: Input bar — prompt, blinking caret, hairline-underline

**Files:**
- Modify: `public/index.html` (replace the `<footer>` block — currently lines 104–121)

**Interfaces:**
- Consumes: `.caret` from Task 1
- Produces: `id="inp"`, `id="sendGlyph"`, `id="promptName"`, `id="promptCaret"`
- Keeps existing `sendMsg()` function untouched

- [ ] **Step 1: Replace the input bar markup**

```html
  <!-- ── Input bar ── -->
  <footer class="input-bar">
    <div class="input-bar__line">
      <span class="caret input-bar__caret" aria-hidden="true"></span>
      <span class="input-bar__name" id="promptName">anonymous</span>
      <span class="input-bar__chev">›</span>
      <input id="inp" type="text" autocomplete="off" spellcheck="false"
             aria-label="message">
      <button id="sendGlyph" type="button" class="input-bar__send" hidden>[ ↵ ]</button>
    </div>
  </footer>
```

- [ ] **Step 2: Add input bar styles**

```css
    /* ── Input bar ────────────────────────────────────────── */
    .input-bar {
      flex: 0 0 auto;
      background: var(--bg-elev);
      border-top: 1px solid var(--line);
      padding: 8px 16px 12px;
    }
    .input-bar__line {
      display: flex;
      align-items: center;
      gap: 8px;
      min-height: 44px;
      padding: 8px 0;
      border-bottom: 1px solid var(--line);
      transition: border-color 120ms ease-out;
    }
    .input-bar__line:focus-within { border-bottom-color: var(--amber); }
    .input-bar__caret { flex: 0 0 auto; }
    .input-bar__name {
      font-size: 11px;
      color: var(--muted);
      font-weight: 500;
    }
    .input-bar__chev {
      color: var(--amber);
      font-weight: 500;
    }
    #inp {
      flex: 1 1 auto;
      min-width: 0;
      font-size: 13px;
      color: var(--text);
      caret-color: transparent;
      padding: 2px 0;
    }
    .input-bar__send {
      font-size: 12px;
      font-weight: 700;
      color: var(--amber);
      padding: 0 8px;
      min-height: 44px;
      transition: opacity 120ms ease-out;
    }
    .input-bar__send[hidden] { display: none; }
```

- [ ] **Step 3: Wire the prompt name + send-glyph visibility**

Replace the existing `sendMsg()` (lines 220–227) with:

```js
  // ── Send text ─────────────────────────────────────────────
  function sendMsg() {
    if (!ME) { toast('set a terminal name first'); return; }
    const inp  = document.getElementById('inp');
    const text = inp.value.trim();
    if (!text) return;
    socket.emit('message', { text, device: ME, type: 'text' });
    inp.value = '';
    document.getElementById('sendGlyph').hidden = true;
  }

  addEventListener('DOMContentLoaded', () => {
    const inp = document.getElementById('inp');
    const send = document.getElementById('sendGlyph');
    const promptName = document.getElementById('promptName');

    inp.addEventListener('input', () => {
      send.hidden = inp.value.length === 0;
    });
    inp.addEventListener('keydown', e => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        sendMsg();
      }
    });
    send.addEventListener('click', sendMsg);

    // Mirror ME into the prompt chip once ME is known (init and after saveName).
    const syncPrompt = () => { if (ME && promptName) promptName.textContent = ME; };
    addEventListener('DOMContentLoaded', syncPrompt);
    // Run once immediately in case init already fired:
    syncPrompt();
  });
```

- [ ] **Step 4: Verify**

Type into the input — send glyph `[ ↵ ]` appears on the right. Press Enter — message sends and glyph disappears. Hairline turns amber while focused. Backspace to empty — glyph hides.

---

### Task 7: Clipboard modal — backdrop, dashed-amber textarea, `[ SEND ↵ ]`

**Files:**
- Modify: `public/index.html` (replace the existing `<div id="clipModal">` block — currently lines 123–160, plus the modal-related JS at lines 230–256)

**Interfaces:**
- Consumes: `--bg-elev`, `--line`, `--amber`, `--muted`, `--bg` from Task 1
- Produces: `id="clipModal"`, `id="clipTA"`, `id="clipCancel"`, `id="clipSend"`
- Keeps `openClip`, `closeClip`, `sendClip` semantics; rewrites internals

- [ ] **Step 1: Replace the clipboard modal markup (lines 123–160)**

```html
  <!-- ── Clipboard modal ── -->
  <div id="clipModal" class="modal" hidden>
    <div class="modal__backdrop" data-close></div>
    <div class="modal__sheet" role="dialog" aria-modal="true" aria-labelledby="clipTitle">
      <header class="modal__head">
        <span class="modal__title">📋  CLIPBOARD.SHARE /tmp/peers</span>
        <button type="button" class="modal__close" data-close aria-label="close">×</button>
      </header>

      <textarea id="clipTA" class="clip-ta" rows="6" spellcheck="false"
        placeholder="paste anything — code, url, notes. ⌘V / Ctrl+V"></textarea>

      <footer class="modal__foot">
        <span class="modal__hint">no network egress · LAN only</span>
        <div class="modal__actions">
          <button id="clipCancel" type="button" class="btn-ghost" data-close>[ CANCEL ]</button>
          <button id="clipSend"   type="button" class="btn-primary clip-send">[ SEND ↵ ]</button>
        </div>
      </footer>
    </div>
  </div>
```

- [ ] **Step 2: Add modal styles**

```css
    /* ── Clipboard modal ───────────────────────────────────── */
    .modal {
      position: fixed;
      inset: 0;
      z-index: 40;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 20px;
    }
    .modal[hidden] { display: none; }
    .modal__backdrop {
      position: absolute;
      inset: 0;
      background: rgba(0, 0, 0, 0.65);
      backdrop-filter: blur(6px);
      -webkit-backdrop-filter: blur(6px);
      animation: rowIn 160ms ease-out both reverse; /* fade-in via opacity */
    }
    .modal__sheet {
      position: relative;
      width: 100%;
      max-width: 520px;
      background: var(--bg-elev);
      border: 1px solid var(--line);
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }
    .modal__head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    }
    .modal__title {
      font-size: 13px;
      font-weight: 500;
      color: var(--text);
    }
    .modal__close {
      width: 28px;
      height: 28px;
      font-size: 18px;
      color: var(--muted);
      line-height: 1;
    }
    .modal__close:hover { color: var(--text); }

    .clip-ta {
      width: 100%;
      min-height: 140px;
      background: var(--bg);
      border: 1px dashed var(--amber);
      padding: 12px 14px;
      font-size: 13px;
      line-height: 1.55;
      color: var(--text);
      resize: vertical;
      transition: border 120ms ease-out;
    }
    .clip-ta:focus { border-style: solid; }

    .modal__foot {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      flex-wrap: wrap;
    }
    .modal__hint {
      font-size: 11px;
      color: var(--muted);
    }
    .modal__actions {
      display: flex;
      gap: 8px;
    }
    .btn-ghost {
      color: var(--muted);
      font-size: 12px;
      font-weight: 500;
      padding: 10px 12px;
      min-height: 44px;
      transition: color 120ms ease-out;
    }
    .btn-ghost:hover { color: var(--text); }
```

- [ ] **Step 3: Replace the modal JS (lines 230–256)**

```js
  // ── Clipboard modal ───────────────────────────────────────
  function openClip() {
    if (!ME) { toast('set a terminal name first'); return; }
    const modal = document.getElementById('clipModal');
    const ta    = document.getElementById('clipTA');
    modal.hidden = false;
    if (navigator.clipboard?.readText) {
      navigator.clipboard.readText()
        .then(t => { if (t) ta.value = t; })
        .catch(() => {});
    }
    setTimeout(() => ta.focus(), 60);
  }

  function closeClip() {
    const modal = document.getElementById('clipModal');
    const ta    = document.getElementById('clipTA');
    modal.hidden = true;
    if (ta) ta.value = '';
  }

  function sendClip() {
    if (!ME) { toast('set a terminal name first'); return; }
    const ta = document.getElementById('clipTA');
    const text = ta.value.trim();
    if (!text) { toast('nothing to send'); return; }
    socket.emit('message', { text, device: ME, type: 'clipboard' });
    closeClip();
    toast('transmitted');
  }

  // Wire open/send/close
  addEventListener('DOMContentLoaded', () => {
    document.getElementById('clipBtn').addEventListener('click', openClip);
    document.getElementById('clipSend').addEventListener('click', sendClip);
    document.getElementById('clipCancel').addEventListener('click', closeClip);

    document.querySelectorAll('#clipModal [data-close]').forEach(el => {
      el.addEventListener('click', e => {
        if (el === e.target || el.dataset.close !== undefined) closeClip();
      });
    });

    document.addEventListener('keydown', e => {
      if (e.key === 'Escape' && !document.getElementById('clipModal').hidden) closeClip();
    });
  });
```

- [ ] **Step 4: Verify**

Click `[ CLIP ]` → modal slides in (opacity). Type or paste into textarea. Press Enter or click `[ SEND ↵ ]` → green row appears in log with `[clip]` chip. ESC closes. Backdrop click closes. Empty submit shows toast `nothing to send`.

---

### Task 8: Toast

**Files:**
- Modify: `public/index.html` (replace the toast `<div>` — currently lines 162–166, and the toast function at lines 326–333)

**Interfaces:**
- Consumes: `--bg-elev`, `--line`, `--text`
- Produces: `id="toast"` with proper transition tokens
- All toast text strings already finalized in the spec §7

- [ ] **Step 1: Replace the toast markup**

```html
  <!-- ── Toast ── -->
  <div id="toast" class="toast" role="status" aria-live="polite"></div>
```

- [ ] **Step 2: Add toast styles**

```css
    /* ── Toast ────────────────────────────────────────────── */
    .toast {
      position: fixed;
      left: 50%;
      bottom: 88px;
      transform: translateX(-50%);
      z-index: 60;
      pointer-events: none;
      background: var(--bg-elev);
      border: 1px solid var(--line);
      color: var(--text);
      font-size: 12px;
      padding: 6px 12px;
      white-space: nowrap;
      opacity: 0;
      transition: opacity 120ms ease-out;
    }
    .toast[data-show="1"] { opacity: 1; }
```

- [ ] **Step 3: Replace the toast function**

Replace lines 326–333 with:

```js
  // ── Toast ─────────────────────────────────────────────────
  let toastTm;
  function toast(text) {
    const el = document.getElementById('toast');
    el.textContent = text;
    el.dataset.show = '1';
    clearTimeout(toastTm);
    toastTm = setTimeout(() => { el.dataset.show = '0'; }, 1800);
  }
```

Also update copy-related toast strings in `copyById` (currently line 263 and 266):

```js
  async function copyById(id, el) {
    const text = txtById[id] ?? '';
    try {
      await navigator.clipboard.writeText(text);
      toast('copied');
      const orig = el.dataset.label || '复制';
      el.dataset.label = orig;
      el.textContent = '✓';
      setTimeout(() => el.textContent = orig, 1500);
    } catch { toast('copy failed'); }
  }
```

Note: we removed the inline "复制" button in Task 5's row design. The `copyById` function is now only used internally by clipboard rows if you re-add a hover button later; this is a defensive update. Verify it is no longer referenced after Task 5 — if not, leave the helper in place but unused.

- [ ] **Step 4: Verify**

Trigger `toast('test')` from devtools. Toast fades in over 120 ms, holds 1800 ms, fades out. Trigger `toast('transmitted')` from a clipboard send — same behavior.

---

### Task 9: Reduced-motion + final accessibility sweep

**Files:**
- Modify: `public/index.html` (the existing `@media (prefers-reduced-motion: reduce)` block already in Task 1 — confirm it covers everything; add small missing rules)

**Interfaces:**
- Audits every keyframe and transition in the file and ensures reduced-motion disables it.

- [ ] **Step 1: Verify global reduced-motion rule**

The Task 1 base style includes:

```css
    @media (prefers-reduced-motion: reduce) {
      *, *::before, *::after {
        animation-duration: 0.001ms !important;
        animation-iteration-count: 1 !important;
        transition-duration: 0.001ms !important;
      }
      .caret, .caret--breathe { animation: none; opacity: 0.85; }
    }
```

Confirm this block is intact.

- [ ] **Step 2: Confirm touch-target sizes**

Open the rendered page and click-test:

- `[ CLIP ]` button — 44 px high minimum ✓ (CSS sets `min-height: 44px`)
- `[ ENTER ↵ ]` button — ✓
- `[ SEND ↵ ]` button — ✓ (uses `.btn-primary` with `min-height: 44px`)
- `[ CANCEL ]` button — change CSS to ensure 44 px:
  - In `.btn-ghost`, the `min-height: 44px` rule is already set.
- The `<input>` and `<textarea>` get the browser default height; add explicit minimums:

Add to the existing input/textarea rules (in the foundation block):

```css
    input, textarea {
      font: inherit;
      color: inherit;
      background: none;
      border: none;
      outline: none;
      min-height: 44px;
    }
```

(Note: this overrides the earlier `input, textarea` declaration; merge into the original block in Task 1 by editing it in place.)

- [ ] **Step 3: Confirm focus rings**

`:focus-visible` is set globally with amber outline at offset 2 px. Verify by tabbing through all interactive elements: caret/divs are not focusable, but buttons and inputs receive a visible amber ring.

- [ ] **Step 4: Confirm body is a flex column**

Open the rendered page in DevTools and confirm:

- `<body>` has `display: flex; flex-direction: column;` (it should, from the original file). If Tailwind classes were removed in Task 1, add inline:

Append to the body's opening tag (currently `<body>`):

```html
<body style="display:flex;flex-direction:column;background:var(--bg);">
```

- [ ] **Step 5: Verify**

Toggle `prefers-reduced-motion: reduce` in DevTools rendering panel. All animations freeze; all transitions become instant; the breathing block sits at `opacity: 0.85` solid.

---

### Task 10: Smoke verify — visual + behavioral checks

**Files:**
- Read-only: `public/index.html`

**No code changes.** Run through every item in spec §11 (Success Criteria). Each must pass before declaring done.

- [ ] **Step 1: No Tailwind utility classes remain**

Run from the project root:

```bash
grep -nE 'class="[^"]*(bg-|text-|flex |grid |p-|m-|w-|h-|rounded|border )' public/index.html
```

Expected output: empty (no matches). If any classes remain, remove them and inline the equivalent CSS or class.

- [ ] **Step 2: No Tailwind CDN reference**

Run:

```bash
grep -n 'cdn.tailwindcss.com' public/index.html
```

Expected: no output.

- [ ] **Step 3: All colors come from the palette**

Run:

```bash
grep -nE '#[0-9A-Fa-f]{6}' public/index.html
```

Expected: matches limited to the seven palette values (`#0A0A0A`, `#111211`, `#1F201D`, `#FFB000`, `#7CFFB2`, `#D9D6CC`, `#5C5D54`) plus `#000` and `#000000` for the backdrop overlay (`rgba(0,0,0,0.65)` is acceptable; `#000000` inside `box-shadow` is acceptable). Any other hex must be replaced with a token.

- [ ] **Step 4: Caret blinking on input**

Open the page, look at the input bar. A solid amber block, 12 × 18 px, blinks between visible and invisible at ~1 Hz (one full cycle per 1.06 s).

- [ ] **Step 5: New message entrance ≤ 100 ms**

Send a message via devtools (`socket.emit('message', { text:'hi', device:'tester', type:'text' })`). The row appears with opacity fading in and a 4 px vertical slide, completing within 100 ms.

- [ ] **Step 6: Boot sequence plays once**

Clear `localStorage`, reload. Watch:

- 0 ms — overlay visible, all three `[..]` indicators cycling.
- ~120 ms — `boot1` switches to `[ OK ]` in green.
- ~240 ms — `boot2` switches to `[ OK ]`.
- `boot3` stays `[ .. ]` until the user connects.

- [ ] **Step 7: Reduced motion freezes everything**

DevTools → Rendering → "Emulate CSS media feature prefers-reduced-motion: reduce". Reload. No blinking, no fade, no translate. Solid blocks remain at full visibility (caret at opacity 0.85).

- [ ] **Step 8: All §8 behaviors preserved**

Manual smoke:

- Save name → reload → name persists, overlay does not reappear.
- Mac UA → name hint is `Alex 的 Mac`.
- Open `[ CLIP ]` on `localhost` → textarea auto-fills clipboard contents.
- Open `[ CLIP ]` on `http://192.168.x.x:4242` → textarea empty (browser security), user pastes manually.
- Hover a foreign row → metadata strip appears.
- Enter sends; Shift+Enter inserts newline.
- ESC closes the clipboard modal.
- Send a 7000-character message → arrives intact on the other device.
- Reboot server → history is empty (existing in-memory ring buffer behavior).

- [ ] **Step 9: Capture screenshots if possible**

If a browser-driven tool is available (e.g. Puppeteer, Playwright), capture three screenshots at 1280×800:

1. Empty state with overlay.
2. Chat view with 4–6 rows mixing own/other/clipboard.
3. Clipboard modal open.

Save under `docs/superpowers/plans/screenshots/` (create the dir) for later review. If not available, skip — visual review is done live in browser.

- [ ] **Step 10: Final commit**

The repo is not git-initialized; confirm with the user whether to `git init` and commit, or leave the working tree as a manual snapshot.

---

## Self-Review

**1. Spec coverage (spec §1–11):**

- §1 brief — covered by overall direction (single file, terminal aesthetic, preserve behavior).
- §2 design language — Task 1 (tokens), Tasks 5/6 (mono + no bubbles).
- §3 tokens — Task 1 (colors + type) and Task 3/4/5/6 (specific component typography).
- §4 signature — Task 1 (`.caret` keyframe), Task 4 (breathing block), Task 6 (input caret).
- §5 component specs — Tasks 2 (setup), 3 (header), 4 (log/empty), 5 (rows), 6 (input), 7 (modal), 8 (toast).
- §6 motion inventory — Task 1 (keyframes), Tasks 2/5/6/7/8 (specific usages), Task 9 (reduced-motion audit).
- §7 copy — Task 7 (modal strings), Task 8 (toast strings), Task 6 (`set a terminal name first`).
- §8 behavior preservation — Task 10 step 8 explicit checklist.
- §9 implementation outline — followed: head rebuilt, Tailwind removed, single `<style>` block, body restructured.
- §10 out of scope — adhered to; no new features, no server changes.
- §11 success criteria — Task 10 enumerates each.

No gaps found.

**2. Placeholder scan:**

- No "TBD", "TODO", "implement later".
- No "Add appropriate error handling" / "fill in details".
- No "Similar to Task N" without full code.
- Every step that touches code shows the full replacement.
- All function/class/id names used in later steps (`#empty`, `#myBadge`, `.caret`, `.row--clip`, `#inp`, `#clipBtn`, `#clipModal`, `#clipTA`, `#clipSend`, `#clipCancel`, `#promptName`, `#sendGlyph`, `#toast`, `.boot-tag`, `#boot1-3`) are introduced explicitly in earlier tasks.

**3. Type / name consistency:**

- IDs introduced once and reused: `#setup`, `#boot1-3`, `#nameIn`, `#startBtn`, `#header`, `#myBadge`, `#clipBtn`, `#msgs`, `#empty`, `#emptyHost`, `#inp`, `#sendGlyph`, `#promptName`, `#clipModal`, `#clipTA`, `#clipCancel`, `#clipSend`, `#toast`.
- CSS classes: `.caret`, `.caret--breathe`, `.setup-card`, `.setup-head`, `.setup-boot`, `.boot-line`, `.boot-dots`, `.boot-tag`, `.setup-label`, `.setup-input`, `.setup-prompt`, `.setup-foot`, `.btn-primary`, `.btn-ghost`, `.app-header`, `.online`, `.device-badge`, `.clip-btn`, `.log`, `.log-empty`, `.row`, `.row--me`, `.row--them`, `.row--clip`, `.row--in`, `.row__head`, `.row__time`, `.row__device`, `.row__rule`, `.row__chip`, `.row__body`, `.row__meta`, `.input-bar`, `.input-bar__line`, `.input-bar__name`, `.input-bar__chev`, `.input-bar__caret`, `.input-bar__send`, `.modal`, `.modal__backdrop`, `.modal__sheet`, `.modal__head`, `.modal__title`, `.modal__close`, `.modal__foot`, `.modal__hint`, `.modal__actions`, `.clip-ta`, `.toast`. All names match between tasks.

No inconsistencies found.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-28-lan-chat-terminal-redesign.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

Also worth noting: this repo is not under git. If we want commits per task (TDD-friendly checkpoints), I should `git init` first. Want me to set that up before execution starts?