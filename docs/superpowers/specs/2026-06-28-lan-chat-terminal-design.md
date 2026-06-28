# LAN Chat — TERMINAL/0 Frontend Redesign

**Date:** 2026-06-28
**Subject:** Visual redesign of `public/index.html` for LAN Chat
**Scope:** Single file (`public/index.html`). No backend changes. No new dependencies.

---

## 1. Brief

LAN Chat is a pure-LAN, zero-account, instant-messenger + clipboard-share tool used by a single geek/dev between their own machines (Mac ↔ Ubuntu). The current frontend is functional but visually generic — a Tailwind-default dark UI with blue/violet bubbles, emoji-as-icon, and no typographic personality. The redesign should make the tool feel like the user's own machine: a terminal that runs in the browser, with phosphor amber on warm black, monospace everything, and a clear visual hook.

**Single job of the page:** Send a message or a clipboard to other devices on the LAN, fast.

**Audience:** The owner themselves, two machines, both running a real terminal daily. Visual idiom should reward that fluency, not soften it.

---

## 2. Design language

> "A local terminal you can open in a browser."

- **Palette:** Phosphor amber + soft green on warm black, six named values.
- **Type:** JetBrains Mono across the entire surface (Latin + digits + UI). CJK falls through to system CJK fonts only where CJK glyphs appear.
- **Shape:** No rounded "bubble" geometry. Messages are log rows. Borders are 1px hairlines in `#1F201D`. No shadows except one soft shadow on the setup card to lift it off the background.
- **Signature:** A 12×18 px solid amber block that blinks at `1.06s steps(2)`, repeated in the input prompt and the empty-state hero. It is the only recurring decorative element.
- **Motion:** Block cursor blink + 100 ms message entrance (`translateY(4px)` + opacity). Nothing else animates.

---

## 3. Tokens

### 3.1 Color (six named values)

| Token        | Hex       | Role                                                    |
| ------------ | --------- | ------------------------------------------------------- |
| `--bg`       | `#0A0A0A` | Page background. Not pure `#000`; retains terminal warmth. |
| `--bg-elev`  | `#111211` | Header, footer, modal surface (one step above bg).      |
| `--line`     | `#1F201D` | Hairlines, borders, divider rules.                      |
| `--amber`    | `#FFB000` | Primary brand. Own messages, active states, send CTA, the blinking block. |
| `--green`    | `#7CFFB2` | Secondary. Online indicator, clipboard messages, success toasts. |
| `--text`     | `#D9D6CC` | Body text. Warm off-white, never `#FFFFFF`.             |
| `--muted`    | `#5C5D54` | Metadata (timestamps, ids, peer badges, log rows).      |

System messages (e.g. error toasts) reuse `--text` on a `#3A1A1A` dim red panel — no extra token; inline value is fine.

### 3.2 Type

- **Family:** `JetBrains Mono` from Google Fonts (preconnect + `?display=swap`), with fallbacks `Menlo, Consolas, "Courier New", monospace`.
- **CJK fallback chain:** appended in the same `font-family` after the Latin stack: `"PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", system-ui, sans-serif`. JetBrains renders Latin; the CJK fallback renders only the CJK glyphs.
- **Scale (px):** `11 / 12 / 13 / 15 / 28`.
- **Weights:** 400 body, 500 metadata (device names, labels), 700 only on the `<h1>` and the `[ SEND ]` CTA. **Italic is forbidden everywhere.**
- **Letter-spacing:** `0` for body; `-0.01em` on the 28 px display headline only.

### 3.3 Layout

- Single full-height column. Header (40 px) → scroll region (`flex: 1`) → input bar (56 px). No sidebars.
- Container max-width: none — fills viewport. Messages are full-width rows; content is left-aligned with a 20 px gutter and reflows naturally.
- Breakpoints: one column for both mobile and desktop. The header collapses the `[ CLIP ]` label to nothing below 420 px wide, keeping only the bracketed glyph `[ · ]`.
- All touch targets ≥ 44 px high.

---

## 4. Signature element

**The blinking amber block.**

- Default size: 12 px × 18 px, solid `--amber`.
- Renders in three places, identically:
  1. Inside the input bar, between the device-name prefix and the typing caret, behaving as the terminal caret itself.
  2. In the top header, immediately before `LAN-CHAT/0`, as a status dot.
  3. In the empty-state, scaled up to 64 px × 64 px, breathing on `2.4s ease-in-out` between opacity `0.35` and `1`.
- Animation:
  - Default: `@keyframes blink { 0%,49%{opacity:1} 50%,100%{opacity:0} }` with `1.06s steps(2) infinite`. Pure on/off, no easing — same cadence as a real terminal caret.
  - Empty-state hero uses a separate `breathe` keyframe (smooth opacity), not the blink, so it doesn't read as a stuck cursor.
- `prefers-reduced-motion: reduce` collapses both to a static solid block at opacity `0.85`.

---

## 5. Component specs

### 5.1 Setup screen (first-run overlay)

Presented as a "boot sequence":

```
╭─ LAN-CHAT/0 ──────────────────────────────── v1.0.0 ─╮
│                                                    │
│  initializing p2p socket ............... [ OK ]    │
│  binding 0.0.0.0:4242 .................. [ OK ]    │
│  awaiting peer handshake ............... [ .. ]    │
│                                                    │
│  identify this terminal:                            │
│  › Alex 的 Mac█                                    │
│                                                    │
│           [ ENTER ↵ ]   connect                    │
│                                                    │
╰────────────────────────────────────────────────────╯
```

- Outer card: `--bg-elev` with a 1 px `--line` border, no border-radius, single soft shadow `0 24px 60px rgba(0,0,0,0.6)` to lift it.
- Boot lines appear sequentially: `[OK]` lines arrive 120 ms apart on mount, the `[..]` line stays animating (3 dots cycling at 600 ms) until the user submits.
- Input is a single underlined field (no box). Focus state: amber underline, 1 px solid.
- The block caret sits inside the input value, blinking as defined in §4.
- ENTER submits; pressing it on an empty name focuses the input and triggers a 120 ms shake (`translateX(-3px 0 3px 0 0)`).

### 5.2 Header

```
▌ LAN-CHAT/0  ● online              [macbook]  [ CLIP ]
```

- Left: block caret + `LAN-CHAT/0` (15 px, weight 500) + a 6 px `--green` round dot + the word `online` in 11 px `--muted`.
- Right: device badge `[<device-name>]` showing the localStorage-saved name, or `[anonymous]` if not yet set (11 px, `--muted`, `--line` border, 4 px padding); and a `[ CLIP ]` button (12 px, weight 500, `--text`, 1 px `--line` border, becomes amber border + amber text on hover).
- On widths < 420 px the device badge and the word `online` are hidden; only the green dot, the brand, and `[ · ]` (a tiny square clipped button) remain.

### 5.3 Message log

- No bubbles. Each message is a row:
  ```
  [14:32:01]  macbook-pro  ────────────
    ls -la
  ```
- Row anatomy:
  - Line 1: timestamp (`--muted`, 11 px) + device name (`--muted`, 11 px, weight 500) + a hairline rule that fills the rest of the row (`--line`, 1 px).
  - Line 2+: message content, indented 12 px (`text-indent`-style spacing, not a literal `padding-left`), 13 px, `--text`. Wraps freely.
- Differentiation:
  - **Own messages:** content rendered in `--amber`.
  - **Others:** content rendered in `--text`.
  - **Clipboard messages:** prefix line gets a `[clip]` chip in `--green`, 11 px on a `--bg` background with a 1 px `--green` border. Content rendered in `--green` and uses the JetBrains Mono stack with `font-feature-settings: "calt" 0, "liga" 0` so URLs/code don't get ligatures.
- Hover on a row reveals a one-line metadata strip appended at the bottom in 10 px `--muted`: `id 8a3f · 42 B`. Appears at `opacity 0→1` over 120 ms. Implementation: a `data-meta` attribute on the row, revealed by `:hover`.
- Entrance animation: `opacity 0 → 1`, `translateY(4px → 0)`, `100ms ease-out`.

### 5.4 Empty state

Centered, 24 px above viewport center:

```
no peers on the wire.
listening on <your LAN IP>:4242


        █
        (the breathing amber block)
```

- Two text lines, 13 px, `--muted`. Second line is the actual IP, monospace.
- Block: 64 × 64 px, `--amber`, `breathe` keyframe (see §4).
- The LAN IP is read from `window.location.host` (works for both `localhost` and LAN access — keeps the message accurate in both cases).

### 5.5 Input bar

```
▌ macbook-pro › ls -la█
─────────────────────────────────────────
```

- Single row, 56 px tall, `--bg-elev` background, 1 px top hairline.
- Composition, left to right: blinking block caret → device name in `--muted` (11 px) → `›` glyph → content text area.
- Text input is invisible (no border, no background); the row's bottom hairline is the visible "underline". Focus state: that hairline turns amber.
- Send affordance: when the input has content, an amber `[ ↵ ]` glyph appears on the far right; otherwise absent. Enter sends.
- Multi-line is supported via Shift+Enter (newline inserted). No separate send button.
- The input's bottom hairline is the visible "underline" for the field. When the input is focused, this hairline switches from `--line` to `--amber`.

### 5.6 Clipboard modal

- Backdrop: `rgba(0,0,0,0.65)` + `backdrop-filter: blur(6px)`.
- Surface: `--bg-elev`, 1 px `--line` border, no radius, padded 24 px.
- Title row: `📋  CLIPBOARD.SHARE /tmp/peers` — the path is decorative terminal flavor, but consistent. Emoji `📋` is allowed here exactly once; it is the only emoji on the page.
- Body: textarea, 14 px, monospace, full width, 6 rows tall, `--bg` background, 1 px dashed `--amber` border (signals "ready to transmit"), focus state collapses dashed border to solid `--amber`.
- Footer: left side, 11 px `--muted` caption "no network egress · LAN only"; right side, `[ CANCEL ]` (text only) and `[ SEND ↵ ]` (amber background, `--bg` text, weight 700). ESC closes.

### 5.7 Toast

- One row, centered horizontally, 24 px above the input bar.
- `--bg-elev` background, 1 px `--line` border, 12 px `--text`, padding `6px 12px`, no radius.
- Fade in 120 ms, hold 1800 ms, fade out 240 ms.

---

## 6. Motion inventory

| Element             | Animation                                         | Duration | Easing              |
| ------------------- | ------------------------------------------------- | -------- | ------------------- |
| Block caret         | opacity `1↔0`                                     | 1.06 s   | `steps(2)`          |
| Empty-state block   | opacity `0.35↔1`                                  | 2.4 s    | `ease-in-out`       |
| Message entrance    | opacity `0→1`, `translateY(4→0)`                  | 100 ms   | `ease-out`          |
| Hover metadata      | opacity `0→1`                                     | 120 ms   | `ease-out`          |
| Setup boot lines    | sequential reveal (120 ms apart)                  | n/a      | n/a                 |
| `[..]` indicator    | three-dot cycle                                   | 600 ms   | `steps(3)`          |
| Modal in/out        | opacity only                                      | 160 ms   | `ease-out`          |
| Setup card shake    | `translateX` keyframe                             | 120 ms   | `ease-in-out`       |

`@media (prefers-reduced-motion: reduce)` disables every keyframe and transition except the static visual state (no fade, no translate).

---

## 7. Copy & microcopy

Voice: terse, system-level, no marketing. Sentence case, present tense for status, imperative for actions.

- Setup title: `LAN-CHAT/0`
- Setup subtitle lines: `initializing p2p socket`, `binding 0.0.0.0:4242`, `awaiting peer handshake`
- Setup prompt: `identify this terminal:`
- Setup button: `[ ENTER ↵ ]   connect`
- Empty state: `no peers on the wire.` / `listening on <host>`
- Input placeholder: never used (the prompt is always visible). If the field is empty after focus, the caret alone signals the prompt.
- Clipboard modal title: `CLIPBOARD.SHARE /tmp/peers`
- Clipboard placeholder: `paste anything — code, url, notes. ⌘V / Ctrl+V`
- Clipboard caption: `no network egress · LAN only`
- Clipboard buttons: `[ CANCEL ]`, `[ SEND ↵ ]`
- Send button (when input has text): `[ ↵ ]`
- Toast on copy success: `copied`
- Toast on copy failure: `copy failed`
- Toast on send without name: `set a terminal name first`
- Toast on empty clipboard send: `nothing to send`
- Toast on clipboard send: `transmitted`

---

## 8. Behavior preservation

The redesign must preserve all existing app behavior:

- Device name persistence in `localStorage` under key `lc-name`.
- Auto-suggested device names by UA (Mac → "Alex 的 Mac", Linux → "Ubuntu", Windows → "我的 PC").
- Auto-read clipboard on modal open when `navigator.clipboard?.readText` is available; silent fallback to manual paste when it isn't.
- WebSocket message protocol unchanged: `{ text, device, type: 'text'|'clipboard' }` and the `history` / `message` events.
- 200-message in-memory ring buffer on the server (unchanged, backend untouched).
- ESC closes the clipboard modal.
- Enter sends; Shift+Enter inserts newline.
- Mobile keyboard and viewport behavior unchanged.

No new features. No new dependencies beyond the JetBrains Mono Google Fonts link.

---

## 9. Implementation outline

Single file edit: `public/index.html`.

1. Replace the `<head>` Google Fonts preconnect + `<link>` to JetBrains Mono.
2. Remove the Tailwind CDN script and its utility classes; rewrite all styles in a single `<style>` block using the tokens above.
3. Restructure the body markup: setup overlay → header → log region → input bar → clipboard modal → toast.
4. Update `render(msg, animate)` to produce the log-row markup described in §5.3, including the hover metadata attribute.
5. Update the setup JS to drive the boot sequence (timed reveal of the two `[OK]` lines, looping `[..]` until submit, shake on empty submit).
6. Add the empty-state and input prompt compositions; wire the breathing block and the blinking caret to the right elements.

No other files change. `server.js`, `package.json`, and the README are untouched in this redesign.

---

## 10. Out of scope

- No new message types, no read receipts, no typing indicators.
- No theme switching, no light mode.
- No background textures, scanlines, or CRT filters.
- No sound effects.
- No server-side changes, no persistence changes.
- No accessibility audit beyond reduced-motion + visible focus rings + 44 px touch targets.
- No build tooling — still a single static HTML file served by the existing Express server.

---

## 11. Success criteria

The redesign is done when:

1. No element on the page uses a Tailwind utility class (Tailwind CDN script removed).
2. Every color in the rendered DOM is one of the six tokens in §3.1.
3. The block caret from §4 is present and blinking on the input bar in its default state.
4. A new incoming message appears within 100 ms of socket delivery, with the row anatomy from §5.3.
5. The setup screen plays the boot sequence exactly once on first visit.
6. `prefers-reduced-motion: reduce` freezes all motion.
7. All existing functionality in §8 still works.