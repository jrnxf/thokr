import { KEY, type Config, type Engine, type EngineState } from "./engine.ts";

export type TyperOptions = {
  config?: Config;
  /** the element that takes a click or tap; the root by default */
  focusEl?: HTMLElement;
  render?: (state: EngineState) => void;
  onFinish?: (state: EngineState) => void;
  afterFinish?: (key: string) => void;
};

/**
 * Keyboard and phone input for one engine. A hidden textarea takes the keys,
 * so phone keyboards work.
 *
 * Once the test has finished, a printable key is not sent on: in a terminal
 * thokr reads r and n there as "again" and "new words", which would reset a
 * block that someone typing fast simply ran past. It goes to `afterFinish`
 * instead. The arrow keys keep their meaning: left starts over, right gets
 * new words.
 */
export class Typer {
  config: Config = {};
  private timer: ReturnType<typeof setTimeout> | undefined;
  private wasDone = false;
  private off: (() => void)[] = [];

  readonly root: HTMLElement;
  readonly ta: HTMLTextAreaElement;
  readonly e: Engine;
  private o: TyperOptions;

  constructor(root: HTMLElement, ta: HTMLTextAreaElement, e: Engine, o: TyperOptions = {}) {
    this.root = root;
    this.ta = ta;
    this.e = e;
    this.o = o;
    const on = <E extends Event>(el: EventTarget, type: string, f: (ev: E) => void) => {
      el.addEventListener(type, f as EventListener);
      this.off.push(() => el.removeEventListener(type, f as EventListener));
    };
    on<PointerEvent>(o.focusEl || root, "pointerdown", (ev) => {
      if ((ev.target as Element).closest("a,button,input,select,label")) return;
      ev.preventDefault();
      this.focus();
    });
    on(ta, "focus", () => root.classList.add("tk-focused"));
    on(ta, "blur", () => root.classList.remove("tk-focused"));
    on<KeyboardEvent>(ta, "keydown", (ev) => {
      if (ev.metaKey || ev.ctrlKey || ev.altKey) return;
      if (ev.key === "Escape") { ta.blur(); ev.preventDefault(); return; }
      if (ev.key === "Tab") return;
      if (ev.key in KEY || ev.key.length === 1) { ev.preventDefault(); this.press(ev.key); }
    });
    on<InputEvent>(ta, "beforeinput", (ev) => {
      ev.preventDefault();
      if (ev.inputType === "deleteContentBackward") this.press("Backspace");
      else if (ev.data) for (const c of ev.data) this.press(c);
    });
    on(ta, "input", () => { ta.value = ""; });
    this.set(o.config || {});
  }
  focus() { this.ta.focus({ preventScroll: true }); }
  set(config: Config) { this.config = { ...this.config, ...config }; this.e.config(this.config); this.after(); }
  press(k: string) {
    if (this.wasDone && !(k in KEY)) { this.o.afterFinish?.(k); return; }
    if (this.wasDone && k === "Backspace") return;
    this.e.key(k);
    this.after();
  }
  after() {
    const s = this.e.state();
    this.o.render?.(s);
    clearTimeout(this.timer);
    const ms = this.e.nextTickMs();
    if (ms >= 0) this.timer = setTimeout(() => { this.e.tick(); this.after(); }, ms);
    if (s.finished && !this.wasDone) { this.wasDone = true; this.o.onFinish?.(s); }
    this.wasDone = s.finished;
  }
  destroy() {
    clearTimeout(this.timer);
    for (const f of this.off) f();
  }
}
