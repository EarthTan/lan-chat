# Input Bar Embed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embed the input bar into the message log: restore the amber native caret, drop the static prefix, drop the elevated panel surface, use a single hairline as the only divider.

**Architecture:** Single-file edit (`public/index.html`). Replace the `.input-bar` CSS block, replace the `<footer class="input-bar">` markup, and remove the now-unreferenced `promptName` JS. No server, no dependencies, no new files.

**Tech Stack:** Vanilla HTML + CSS + JS. JetBrains Mono via Google Fonts. Socket.IO (unchanged).

## Global Constraints

These apply to every task. They are copied verbatim from the spec `docs/superpowers/specs/2026-06-28-lan-chat-input-bar-embed.md` and the prior design `docs/superpowers/specs/2026-06-28-lan-chat-terminal-design.md`.

- **Single file edit:** `public/index.html` only. No other files change.
- **Six color tokens only:** `--bg #0A0A0A`, `--bg-elev #111211`, `--line #1F201D`, `--amber #FFB000`, `--green #7CFFB2`, `--text #D9D6CC`, `--muted #5C5D54`. No new tokens.
- **Type:** JetBrains Mono across the surface. 11 / 12 / 13 / 15 / 28 px scale. Italic forbidden.
- **Easing:** `--ease: cubic-bezier(0.2, 0, 0, 1)` — used by every transition.
- **Touch targets:** ≥ 44 px high for interactive elements.
- **Reduced motion:** `prefers-reduced-motion: reduce` disables keyframes/transitions (already in place; do not regress).
- **Behavior preserved:** Enter sends; Shift+Enter inserts newline; IME composition (`compositionstart` / `compositionend` / `e.isComposing`) respected; WebSocket protocol unchanged; `localStorage` key `lc-name` unchanged.
- **JS trim scope:** only the `promptName` const, the `syncPrompt` function, and its call site are removed. Nothing else in the `<script>` block changes.
- **Commit message style:** `feat(terminal/0): …` for changes, `chore(terminal/0): …` for verification / cleanup, matching the existing terminal/0 commit log.

---

### Task 1: Replace `.input-bar` CSS block

**Files:**
- Modify: `public/index.html:439-499` (the entire `.input-bar` style block, from the `/* ── Input bar ─── */` comment through `.input-bar__count[data-warn="1"]`).

**Interfaces:**
- Consumes: existing color tokens (`--bg`, `--line`, `--amber`, `--text`, `--muted`), existing `--ease` cubic-bezier, existing font stack `--mono`.
- Produces: a single `.input-bar` rule with `--bg` background and a `::after` hairline pseudo-element; an `.input-bar__row` flex wrapper; `#inp` with `caret-color: var(--amber)`; `.input-bar__send` and `.input-bar__count` re-anchored as flex items.

- [ ] **Step 1: Delete the existing `.input-bar` CSS block**

In `public/index.html`, locate the comment `/* ── Input bar ────────────────────────────────────────── */` at line 439. Delete everything from that comment line through the line `.input-bar__count[data-warn="1"] { color: var(--amber); }` (line 499), inclusive. Verify deletion with a grep that returns zero matches:

```bash
grep -c "\.input-bar__count\[data-warn" public/index.html
```

Expected output: `0`.

- [ ] **Step 2: Insert the new `.input-bar` CSS block**

In place of the deleted block (immediately before the `/* ── Clipboard modal ── */` comment that follows), insert:

```css
    /* ── Input bar — embedded in the log ────────────────── */
    .input-bar {
      flex: 0 0 auto;
      background: var(--bg);
      padding: 12px 20px 16px;
      position: relative;
    }
    /* The only separator from the log: a hairline that turns amber on focus. */
    .input-bar::after {
      content: "";
      position: absolute;
      left: 20px;
      right: 20px;
      top: 0;
      height: 1px;
      background: var(--line);
      transition: background 140ms var(--ease);
    }
    .input-bar:focus-within::after { background: var(--amber); }

    .input-bar__row {
      display: flex;
      align-items: center;
      flex-wrap: wrap;
      gap: 4px 8px;
    }

    #inp {
      flex: 1 1 auto;
      min-width: 0;
      font-size: 13px;
      color: var(--text);
      caret-color: var(--amber);
      padding: 2px 0;
      min-height: 22px;
      line-height: 1.55;
    }

    .input-bar__send {
      flex: 0 0 auto;
      font-size: 12px;
      font-weight: 700;
      color: var(--amber);
      padding: 0;
      min-height: 44px;
      opacity: 0.25;
      transition: opacity 140ms var(--ease);
    }
    .input-bar__send[data-ready="1"] { opacity: 1; }

    .input-bar__count {
      flex: 0 0 auto;
      font-size: 11px;
      color: var(--muted);
      opacity: 0;
      transition: opacity 140ms var(--ease);
      pointer-events: none;
    }
    .input-bar__count[data-show="1"] { opacity: 1; }
    .input-bar__count[data-warn="1"] { color: var(--amber); }
```

- [ ] **Step 3: Verify the new selectors are present and the old ones are gone**

Run, in order, and confirm each returns at least one match for the first three and zero for the last six:

```bash
grep -c "input-bar__row"               public/index.html   # expect ≥ 1
grep -c "caret-color: var(--amber)"    public/index.html   # expect ≥ 1
grep -c "input-bar:focus-within::after" public/index.html # expect 1
grep -c "\.input-bar__line"            public/index.html   # expect 0
grep -c "\.input-bar__caret"           public/index.html   # expect 0
grep -c "\.input-bar__name"            public/index.html   # expect 0
grep -c "\.input-bar__chev"            public/index.html   # expect 0
grep -c "input-bar__send\[hidden\]"    public/index.html   # expect 0
grep -c "background: var(--bg-elev)"    public/index.html   # expect 0  (the .input-bar block, not elsewhere)
```

Note: `grep -c "background: var(--bg-elev)"` may also match `.setup-card`, `.modal__sheet`, `.toast`, `.app-header`, `.clip-ta` (the textarea), and `.input-bar` (which we just rewrote). Verify the only matches are NOT `.input-bar` by also running:

```bash
grep -n "background: var(--bg-elev)" public/index.html
```

Expected matches are in `.setup-card`, `.app-header`, `.modal__sheet`, `.toast` — NOT in `.input-bar`. If `.input-bar` appears in this list, the deletion in Step 1 missed a line.

- [ ] **Step 4: Commit**

```bash
git add public/index.html
git commit -m "feat(terminal/0): input bar — embed in log, restore amber native caret

Drop .input-bar__line / __caret / __name / __chev. Single hairline via
::after. Background matches the log (var(--bg)). Native caret tinted
amber so the cursor is visible at the typing position.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Replace input bar DOM and remove `promptName` JS

**Files:**
- Modify: `public/index.html:681-691` (the `<footer class="input-bar">…</footer>` block).
- Modify: `public/index.html` JS block — the `promptName` const at the start of the DOMContentLoaded handler around line 850, the `syncPrompt` function and its call site around line 876.

**Interfaces:**
- Consumes: the new `.input-bar__row` CSS class from Task 1; the IDs `#inp`, `#sendGlyph`, `#inpCount` (preserved).
- Produces: a footer that contains exactly one flex row with the input, the count, and the send glyph — no prefix elements. No remaining references to `promptName` or `input-bar__name` in the script.

- [ ] **Step 1: Replace the input bar markup**

In `public/index.html`, locate the line `<!-- ── Input bar ── -->` followed by `<footer class="input-bar">`. Replace the entire `<footer>…</footer>` element with:

```html
  <!-- ── Input bar ── -->
  <footer class="input-bar">
    <div class="input-bar__row">
      <input id="inp" type="text" autocomplete="off" spellcheck="false"
             aria-label="message">
      <span id="inpCount" class="input-bar__count" aria-hidden="true"></span>
      <button id="sendGlyph" type="button" class="input-bar__send" aria-label="send message">[ ↵ ]</button>
    </div>
  </footer>
```

Verify the removed elements are gone:

```bash
grep -c "input-bar__caret"  public/index.html   # expect 0
grep -c "input-bar__name"   public/index.html   # expect 0
grep -c "input-bar__chev"   public/index.html   # expect 0
grep -c "id=\"promptName\"" public/index.html   # expect 0
```

- [ ] **Step 2: Remove `promptName` from the DOMContentLoaded handler**

Find this block inside the second `addEventListener('DOMContentLoaded', …)` (the one that wires `#inp`):

```js
    const inp   = document.getElementById('inp');
    const send  = document.getElementById('sendGlyph');
    const count = document.getElementById('inpCount');
    const promptName = document.getElementById('promptName');
```

Delete the line `const promptName = document.getElementById('promptName');`. The result should be:

```js
    const inp   = document.getElementById('inp');
    const send  = document.getElementById('sendGlyph');
    const count = document.getElementById('inpCount');
```

- [ ] **Step 3: Remove the `syncPrompt` function and its call**

Find the lines:

```js
    const syncPrompt = () => { if (ME && promptName) promptName.textContent = ME; };
    syncPrompt();
```

Delete both lines entirely.

Verify no leftover references in the script:

```bash
grep -n "promptName" public/index.html   # expect zero matches
grep -n "syncPrompt" public/index.html   # expect zero matches
```

- [ ] **Step 4: Verify the script still has its required handlers**

```bash
grep -n "sendMsg\|compositionstart\|compositionend\|isComposing\|data-ready\|data-show\|data-warn" public/index.html
```

Expected: matches for every identifier above (they all live in the input bar's JS handlers and must remain).

- [ ] **Step 5: Commit**

```bash
git add public/index.html
git commit -m "feat(terminal/0): input bar — drop prompt prefix, single flex row

Remove input-bar__caret / __name / __chev and the promptName JS.
Wrap the input, char count, and send glyph in .input-bar__row.
Device identity stays in the header badge.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Static + smoke verification

**Files:**
- Read-only: `public/index.html`.

**Interfaces:**
- Consumes: the final state of `public/index.html` from Tasks 1 + 2.
- Produces: a verification report. Any failure blocks merge.

- [ ] **Step 1: Token compliance check**

Every color in the rendered DOM must be one of the six tokens plus `#000` for the body fallback. Run:

```bash
grep -oE "#[0-9A-Fa-f]{3,6}" public/index.html | sort -u
```

Expected unique colors: at most `#0A0A0A`, `#111211`, `#1F201D`, `#FFB000`, `#7CFFB2`, `#D9D6CC`, `#5C5D54`, `#FFFFFF` (the `:focus-visible` outline offset is `2px` and the outline itself uses `var(--amber)`; `#FFFFFF` should NOT appear — if it does, surface the line). `#000000` is also acceptable only if used for `caret-color` or `text-decoration`; verify any such match is intentional.

If any color outside the six tokens appears, fix it before proceeding.

- [ ] **Step 2: Behavioral surface check**

Confirm none of the prior behaviors were lost. Each grep must return at least one match:

```bash
grep -c "Enter'\s*&&\s*!e\.shiftKey"        public/index.html   # Enter guard
grep -c "imeComposing"                     public/index.html   # IME flag
grep -c "compositionstart"                 public/index.html   # IME listener
grep -c "compositionend"                   public/index.html   # IME listener
grep -c "navigator.clipboard?.readText"    public/index.html   # clipboard auto-read
grep -c "localStorage.getItem('lc-name')"  public/index.html   # name persistence
grep -c "Escape'\s*&&\s*!document"         public/index.html   # ESC closes modal
```

Each expected: ≥ 1.

- [ ] **Step 3: Server smoke**

Boot the server in the background, confirm it serves the page with HTTP 200, and confirm the served body contains the new structure:

```bash
node server.js &
SERVER_PID=$!
sleep 1
curl -fsS -o /tmp/lanchat.html -w "%{http_code}\n" http://localhost:4242/ | grep -q "^200$" \
  && echo "HTTP 200 OK" \
  || (kill $SERVER_PID; echo "FAILED"; exit 1)
grep -q "input-bar__row"   /tmp/lanchat.html && echo "row wrapper served OK"
grep -q "caret-color: var(--amber)" /tmp/lanchat.html && echo "amber native caret served OK"
grep -q "id=\"inp\""       /tmp/lanchat.html && echo "#inp served OK"
kill $SERVER_PID
```

Expected: three `… served OK` lines, no `FAILED`.

- [ ] **Step 4: a11y spot check**

Verify the input still has its accessible name, the send button still has `aria-label`, and the count is still `aria-hidden`:

```bash
grep -q 'aria-label="message"'               public/index.html && echo "input label OK"
grep -q 'aria-label="send message"'          public/index.html && echo "send label OK"
grep -q 'id="inpCount".*aria-hidden="true"'  public/index.html && echo "count hidden OK"
```

Each expected: `… OK`.

- [ ] **Step 5: Commit verification log (optional but encouraged)**

```bash
git add public/index.html
git commit --allow-empty -m "chore(terminal/0): input bar embed — verification passed

Token compliance: only the six design tokens present.
Behavioral surface: Enter guard, IME listeners, clipboard read,
name persistence, ESC-close all intact.
Server smoke: HTTP 200, new DOM and CSS served.
a11y: input and send labels present, count still aria-hidden.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

(If nothing changed in this step, the `--allow-empty` flag still records the verification audit trail.)

---

## Self-Review Notes

**Spec coverage:**

| Spec section / requirement                                   | Covered by       |
| ------------------------------------------------------------ | ---------------- |
| §3.1 — new CSS for `.input-bar`, `.input-bar::after`, `.input-bar__row`, `#inp`, `.input-bar__send`, `.input-bar__count` | Task 1           |
| §3.2 — DOM rewrite: drop caret/name/chevron; wrap in `__row`; preserve `#inp`, `#inpCount`, `#sendGlyph` | Task 2           |
| §3.3 — JS: remove `syncPrompt` and `promptName` const       | Task 2           |
| §3.4 — no new animations, only 140 ms `background` transition | Task 1 (CSS)     |
| §4 — header / log / setup / modal / toast / tokens / behavior preserved | Task 3 (smoke + grep) |
| §6 — IME, long messages, mobile, focus rings                | Task 3 (a11y + smoke) |
| §7 — single-file edit, three numbered steps                 | Tasks 1 + 2 + 3  |
| §8 — success criteria #1–10                                 | Task 3 (verification) |

**Placeholder scan:** No `TODO`, `TBD`, "add appropriate", "implement later" patterns. Every step contains concrete code, exact grep commands, and expected outputs.

**Type / name consistency:** `promptName`, `syncPrompt`, `.input-bar__line`, `.input-bar__caret`, `.input-bar__name`, `.input-bar__chev`, `.input-bar__send[hidden]` are referenced in deletions only and verified gone. New names (`.input-bar::after`, `.input-bar__row`) are introduced in Task 1 and consumed by Task 2's DOM and Task 3's grep checks. No naming collisions.
