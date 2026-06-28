# LAN Chat — Input Bar Embedding (Detail Polish)

**Date:** 2026-06-28
**Subject:** Detail-level polish for the input bar in `public/index.html`
**Scope:** Single file (`public/index.html`). No backend changes. No new dependencies.

This spec revises the input bar component from `docs/superpowers/specs/2026-06-28-lan-chat-terminal-design.md` §5.5 only. Every other component keeps the prior spec unchanged.

---

## 1. Brief

The current input bar reads as a boxed-in panel disconnected from the message log above it. Two problems:

1. **No visible cursor in the typing area.** The native browser caret is hidden via `caret-color: transparent`, and the decorative amber block caret is a static prefix glyph on the left. When the user focuses the field and starts typing, there is no visual feedback for *where* the next character will land.
2. **Visual disconnection from the log.** The bar uses `--bg-elev`, a 1px top border, an internal `.input-bar__line` with padding + bottom border, plus three left-aligned prefix elements (block caret, device name, `›` chevron). Together these read as a framed panel — a "compose box" UI, not a continuation of the log stream.

The user wants the input bar to feel **embedded**: just another row in the log, with the same background and column rhythm as the messages above it. Identity already lives in the header device badge, so the prompt prefix is redundant.

---

## 2. Direction

**Treat the input as a row, not a panel.** Same `--bg` as the log. Single hairline as the only separator. Amber native caret as the only cursor. Device identity stays in the header.

### Before

```
┌─────────────────────────────────────────────────┐
│  ▌  macbook-pro › ls -la█            [ ↵ ]      │   ← three prefix elements
│ ─────────────────────────────────────────────── │   ← internal bottom border
│           (--bg-elev)                1234 / 8000 │
└─────────────────────────────────────────────────┘
       (--line top border separates from log)
```

### After

```
log region
─────────────────────────────────────────────────
ls -la█                                    [ ↵ ]
─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─
            (--bg, only a 1px hairline at the bottom)
```

---

## 3. Concrete changes

### 3.1 CSS

**Replace** `.input-bar`, `.input-bar__line`, `.input-bar__caret`, `.input-bar__name`, `.input-bar__chev` styles with a single set:

```css
/* ── Input bar — embedded in the log ────────────────── */
.input-bar {
  flex: 0 0 auto;
  background: var(--bg);          /* was --bg-elev — matches log surface */
  padding: 12px 20px 16px;        /* matches .log padding for column alignment */
  position: relative;
}

.input-bar::after {
  /* the only visible separator — a hairline above */
  content: "";
  position: absolute;
  left: 20px;
  right: 20px;
  top: 0;
  height: 1px;
  background: var(--line);
  transition: background 140ms var(--ease);
}
.input-bar:focus-within::after {
  background: var(--amber);
}

#inp {
  display: block;
  width: 100%;
  font-size: 13px;
  color: var(--text);
  caret-color: var(--amber);      /* was: transparent */
  padding: 2px 0;
  min-height: 22px;
  line-height: 1.55;
}

.input-bar__send {
  /* existing rules retained — see Send glyph notes below */
  flex: 0 0 auto;
}
.input-bar__count {
  /* existing rules retained — see Char count notes below */
  flex: 0 0 auto;
}
```

**Structural rewrite:** wrap the input + send glyph + char count in a single flex row `.input-bar__row` so the send button and char count align naturally on the right:

```css
.input-bar__row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px 8px;
}
#inp              { flex: 1 1 auto; min-width: 0; }
.input-bar__send  { flex: 0 0 auto; }
.input-bar__count { flex: 0 0 auto; }
```

**Send glyph** (`[ ↵ ]`) — existing rules retained, just consumed by the flex row:
- Color `--amber`, font-weight 700, 12 px.
- Idle: `opacity: 0.25` (present, but quiet).
- Ready (`data-ready="1"`): `opacity: 1`.
- No left padding — sits directly on the text baseline via `flex: 0 0 auto`.

**Char count** — existing rules retained, repositioned from absolute to inline flex item:
- 11 px `--muted`.
- Inline (flex item with `flex: 0 0 auto`) instead of absolutely positioned at `bottom: 4px`.
- Surfaces only at `len ≥ 6000` with `data-show="1"`.
- `data-warn="1"` (color `--amber`) at `len > 7500`.
- On mobile widths (`max-width: 420px`), the count stays visible — the row has room.

### 3.2 DOM

**Remove** these three elements from the input bar:
- `<span class="caret input-bar__caret" ...>`
- `<span class="input-bar__name" id="promptName">anonymous</span>`
- `<span class="input-bar__chev">›</span>`

**Wrap** the remaining elements in a single row container:

```html
<footer class="input-bar">
  <div class="input-bar__row">
    <input id="inp" type="text" autocomplete="off" spellcheck="false"
           aria-label="message">
    <span id="inpCount" class="input-bar__count" aria-hidden="true"></span>
    <button id="sendGlyph" type="button" class="input-bar__send" aria-label="send message">[ ↵ ]</button>
  </div>
</footer>
```

### 3.3 JS

**Remove** the `syncPrompt` function and the reference to `promptName` (no longer in the DOM). The device identity is already reflected in `#myBadge` in the header.

**Keep** everything else in the input handlers:
- IME composition tracking (`compositionstart` / `compositionend`).
- `input` listener updating `send.dataset.ready` and `count.dataset.show` / `warn`.
- `keydown` listener with `Enter && !e.shiftKey && !imeComposing && !e.isComposing` guard.
- `sendMsg()` body unchanged.
- `[ ↵ ]` click handler unchanged.

### 3.4 Animation

No new animations. The only transition introduced is the 140 ms `background` change on `.input-bar::after` when the input gains/loses focus — same easing (`--ease`) as existing transitions.

---

## 4. What does NOT change

- **Header** (§5.2 of prior spec): unchanged. The amber block caret still sits before `LAN-CHAT/0` as the brand glyph.
- **Log region** (§5.3, §5.4): unchanged. Messages and empty state intact.
- **Setup overlay** (§5.1): unchanged. The amber block caret still blinks inside the setup input value.
- **Clipboard modal** (§5.6), **toast** (§5.7): unchanged.
- **Tokens** (§3 of prior spec): all six colors reused. No new tokens.
- **Behavior preservation** (§8 of prior spec): all behaviors hold — Enter sends, Shift+Enter newline, IME composition respected, 200-message ring buffer untouched, WebSocket protocol unchanged.

---

## 5. Signature element note

The amber block caret (prior spec §4) now lives in **two** places instead of three:

1. Top header, before `LAN-CHAT/0` — brand glyph.
2. Setup overlay input value — the cursor while naming the device.
3. ~~Input bar~~ — removed. The input bar's signature is now the *blinking amber native caret* + amber hairline on focus, which is more authentic to a real terminal anyway.

This is a deliberate reduction in signature density, traded for stronger visual continuity between input and log. The block caret still has two recurrences, which is enough to anchor the brand.

---

## 6. Risk and edge cases

- **IME composition (CJK input):** Native caret is restored. Composition indicator position is the browser's responsibility; we don't measure or replicate it. Behavior should match any other text input on the web.
- **Long messages:** Char count at 6000 chars surfaces inline on the right. No overlap with `[ ↵ ]` because the input flexes (`flex: 1 1 auto; min-width: 0`) and shrinks to make room.
- **Mobile:** Below 420 px the input row still has enough horizontal room for an inline char count + send glyph. `.input-bar__row` carries `flex-wrap: wrap;` so the count and send glyph wrap cleanly to a second line on very narrow widths (< 360 px) without overflowing.
- **Focus rings:** Existing `:focus-visible` outline rule (1px `--amber`, 2px offset) still applies to the input on keyboard navigation. Touch focus keeps the amber hairline state.

---

## 7. Implementation outline

Single file edit: `public/index.html`.

1. Replace the `.input-bar` block in the `<style>` with the new rules (§3.1).
2. Replace the `<footer class="input-bar">…</footer>` markup (§3.2).
3. In the JS, remove the `promptName` const, the `syncPrompt` function, and its call site. Nothing else changes in the script.

`server.js`, `package.json`, `README.md` untouched.

---

## 8. Success criteria

The change is done when:

1. Focusing the input shows an amber blinking caret at the actual typing position (verified by typing a character — the caret appears to its right).
2. The input bar has no visible elevated surface; its background matches the log region above.
3. The only divider between log and input is a single 1 px hairline that turns amber on focus.
4. The static amber block caret prefix, the device-name prefix, and the `›` chevron are gone from the input bar.
5. The device name still appears in the header `#myBadge` (unchanged).
6. Enter sends; Shift+Enter inserts newline; IME composition is respected.
7. The send glyph `[ ↵ ]` still appears at opacity 0.25 idle / 1.0 ready.
8. Char count surfaces inline at ≥ 6000 chars, warn color at > 7500.
9. The amber block caret still appears in the header (before `LAN-CHAT/0`) and inside the setup overlay input value.
10. No new files, no new dependencies, no server changes.
