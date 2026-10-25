// thokr's own Rust (src/thok.rs, ui.rs, layout.rs, lang.rs), compiled to
// WebAssembly for wasm32-wasip1 by web/build.sh. This file is the WASI shim
// it needs (a clock, random bytes, stderr) and the typing loop that sends it
// keys and ticks, as main.rs does in a terminal.

const KEY = { Backspace: 0x110001, ArrowLeft: 0x110002, ArrowRight: 0x110003 };
const LANGS = { english: 0, english1k: 1, english10k: 2 };
// build.sh names the engine by its content
const WASM = new URL("./__THOKR_WASM__", import.meta.url);

let compiled;
async function compile(url) {
  compiled ??= (async () => {
    if (typeof process !== "undefined" && url.protocol === "file:") {
      const { readFile } = await import("node:fs/promises");
      return WebAssembly.compile(await readFile(url));
    }
    return WebAssembly.compileStreaming(fetch(url));
  })();
  return compiled;
}

/** One running copy of thokr. Several can run side by side. */
export async function loadEngine(url = WASM) {
  let mem;
  const view = () => new DataView(mem.buffer);
  const wasi = {
    clock_time_get(id, _precision, out) {
      const ns = id === 0 ? BigInt(Date.now()) * 1000000n : BigInt(Math.round(performance.now() * 1e6));
      view().setBigUint64(out, ns, true);
      return 0;
    },
    random_get(ptr, len) {
      crypto.getRandomValues(new Uint8Array(mem.buffer, ptr, len));
      return 0;
    },
    environ_sizes_get(count, size) {
      view().setUint32(count, 0, true);
      view().setUint32(size, 0, true);
      return 0;
    },
    environ_get() { return 0; },
    fd_write(_fd, iovs, n, written) {
      let total = 0, text = "";
      for (let i = 0; i < n; i++) {
        const p = view().getUint32(iovs + i * 8, true), l = view().getUint32(iovs + i * 8 + 4, true);
        text += new TextDecoder().decode(new Uint8Array(mem.buffer, p, l));
        total += l;
      }
      if (text.trim()) console.warn("thokr:", text.trim());
      view().setUint32(written, total, true);
      return 0;
    },
    proc_exit(code) { throw new Error(`thokr exited with ${code}`); },
  };
  const instance = await WebAssembly.instantiate(await compile(url), { wasi_snapshot_preview1: wasi });
  const x = instance.exports;
  mem = x.memory;
  const read = (len) => JSON.parse(new TextDecoder().decode(new Uint8Array(mem.buffer, x.tk_out(), len)));
  return {
    /** The flags, as thokr takes them; 0 or "" means not given. */
    config({ words = 15, sentences = 0, secs = 0, pace = 0, lang = "english", prompt = "" } = {}) {
      const b = new TextEncoder().encode(prompt || "");
      if (b.length) new Uint8Array(mem.buffer, x.tk_inbox(b.length), b.length).set(b);
      x.tk_config(words, sentences || 0, secs || 0, pace || 0, LANGS[lang] ?? 0, b.length);
    },
    key(k) { x.tk_key(typeof k === "number" ? k : KEY[k] ?? k.codePointAt(0)); },
    tick() { x.tk_tick(); },
    nextTickMs() { return x.tk_next_tick_ms(); },
    /** thokr's screen at w x h cells: {w, h, cursor, rows[y][x] = 0 | [symbol, fg, modifiers]} */
    frame(w, h) { return read(x.tk_render(w, h)); },
    /** {prompt, input ("c"/"x" per key), started, finished, pace, remaining, wpm, accuracy, sd, elapsed, coords} */
    state() { return read(x.tk_state()); },
  };
}

/**
 * Keyboard and phone input for one engine. A hidden textarea takes the keys,
 * so phone keyboards work.
 *
 * Once the test has finished, a printable key is not sent on: in a terminal
 * thokr reads r and n there as "again" and "new words", which would reset a
 * block that someone typing fast simply ran past. It goes to `afterFinish`
 * instead. The arrow keys keep their meaning: left starts over, right gets
 * new words.
 *
 * opts: { config, render(engine, state), onFinish(state), afterFinish(key), focusEl }
 */
export class WasmTyper {
  constructor(root, engine, opts = {}) {
    this.root = root;
    this.e = engine;
    this.o = opts;
    const ta = (this.ta = document.createElement("textarea"));
    Object.assign(ta, { autocapitalize: "off", autocomplete: "off", spellcheck: false, name: "thokr-input", className: "tk-input" });
    ta.setAttribute("autocorrect", "off");
    ta.setAttribute("aria-label", "Type the prompt here");
    root.appendChild(ta);
    (opts.focusEl || root).addEventListener("pointerdown", (ev) => {
      if (ev.target.closest("a,button,input,select,label")) return;
      ev.preventDefault();
      this.focus();
    });
    ta.addEventListener("focus", () => root.classList.add("tk-focused"));
    ta.addEventListener("blur", () => root.classList.remove("tk-focused"));
    ta.addEventListener("keydown", (ev) => {
      if (ev.metaKey || ev.ctrlKey || ev.altKey) return;
      if (ev.key === "Escape") { ta.blur(); ev.preventDefault(); return; }
      if (ev.key === "Tab") return;
      if (ev.key in KEY || ev.key.length === 1) { ev.preventDefault(); this.press(ev.key); }
    });
    ta.addEventListener("beforeinput", (ev) => {
      ev.preventDefault();
      if (ev.inputType === "deleteContentBackward") this.press("Backspace");
      else if (ev.data) for (const c of ev.data) this.press(c);
    });
    ta.addEventListener("input", () => { ta.value = ""; });
    this.set(opts.config || {});
  }
  focus() { this.ta.focus({ preventScroll: true }); }
  set(config) { this.config = { ...this.config, ...config }; this.e.config(this.config); this.after(); }
  press(k) {
    if (this.wasDone && !(k in KEY)) { this.o.afterFinish?.(k); return; }
    if (this.wasDone && k === "Backspace") return;
    this.e.key(k);
    this.after();
  }
  after() {
    const s = this.e.state();
    this.o.render?.(this.e, s);
    clearTimeout(this.timer);
    const ms = this.e.nextTickMs();
    if (ms >= 0) this.timer = setTimeout(() => { this.e.tick(); this.after(); }, ms);
    if (s.finished && !this.wasDone) { this.wasDone = true; this.o.onFinish?.(s); }
    this.wasDone = s.finished;
  }
}

/** Paints a prompt from the engine's state into one span per char: c correct,
 *  x incorrect, u untyped, cur where the next key goes, pace for the pace caret. */
export function paintState(el, s) {
  const chars = Array.from(s.prompt);
  if (el._prompt !== s.prompt || el.children.length !== chars.length) {
    el._prompt = s.prompt;
    el.textContent = "";
    for (const ch of chars) {
      const span = document.createElement("span");
      span.textContent = ch;
      el.appendChild(span);
    }
  }
  const kids = el.children;
  for (let i = 0; i < kids.length; i++) {
    const o = s.input[i];
    let cls = o ? (o === "c" ? "c" : "x") : "u";
    // thokr shows a mistyped space as a dot
    const shown = o === "x" && chars[i] === " " ? "·" : chars[i];
    if (kids[i].textContent !== shown) kids[i].textContent = shown;
    if (i === s.input.length) cls += " cur";
    if (s.pace === i) cls += " pace";
    if (kids[i].className !== cls) kids[i].className = cls;
  }
  el.classList.toggle("tk-end", s.input.length === chars.length);
}

/**
 * Typing with no click: the prompt nearest the middle of the window takes
 * the keyboard, from page load on. Keys pressed while nothing has focus
 * (after a link, a button or Esc) go to it too. `entries` is [{ typer, el }]
 * in page order. A phone still needs one tap: browsers open the keyboard only
 * for a touch.
 */
export function autoType(entries) {
  let active = null;
  const activate = (entry) => {
    if (entry === active) return;
    active?.el.classList.remove("tk-active");
    active = entry;
    entry.el.classList.add("tk-active");
    const fe = document.activeElement;
    if (!fe || fe === document.body || fe.classList?.contains("tk-input")) entry.typer.focus();
  };
  const pick = () => {
    const mid = innerHeight / 2;
    let best = entries[0], bestD = Infinity;
    for (const e of entries) {
      const r = e.el.getBoundingClientRect();
      if (r.bottom < 0 || r.top > innerHeight) continue;
      const d = Math.abs((r.top + r.bottom) / 2 - mid);
      if (d < bestD) { bestD = d; best = e; }
    }
    activate(best);
  };
  addEventListener("scroll", () => requestAnimationFrame(pick), { passive: true });
  addEventListener("resize", pick);
  document.addEventListener("keydown", (ev) => {
    const fe = document.activeElement;
    if (fe && fe !== document.body && fe.matches("input,textarea,select,button,a,[contenteditable]")) {
      if (!(fe.matches("button,a") && ev.key.length === 1 && ev.key !== " ")) return;
    }
    if (ev.metaKey || ev.ctrlKey || ev.altKey || !active) return;
    if (ev.key.length === 1 || ["Backspace", "ArrowLeft", "ArrowRight"].includes(ev.key)) {
      ev.preventDefault();
      active.typer.focus();
      active.typer.press(ev.key);
    }
  });
  // a click or tap on a prompt makes it the active one
  for (const e of entries) e.typer.ta.addEventListener("focus", () => activate(e));
  pick();
  entries[0].typer.focus();
  return { activate };
}
