import type { Typer } from "./typer.ts";

export type Entry = { typer: Typer; el: HTMLElement };

/**
 * Typing with no click: the prompt nearest the middle of the window takes
 * the keyboard, from page load on. Keys pressed while nothing has focus
 * (after a link, a button or Esc) go to it too. `entries` is [{ typer, el }]
 * in page order. A phone still needs one tap: browsers open the keyboard only
 * for a touch. `hold(entry)` keeps the keyboard on `entry` while the page
 * scrolls to it: until it is the nearest, the scroll ends, the user scrolls,
 * or a second has gone by.
 */
export function autoType(entries: Entry[]) {
  let active: Entry | null = null, held: Entry | null = null, holdTimer: ReturnType<typeof setTimeout> | undefined;
  const off: (() => void)[] = [];
  const on = (el: EventTarget, type: string, f: (ev: any) => void, opts?: AddEventListenerOptions) => {
    el.addEventListener(type, f, opts);
    off.push(() => el.removeEventListener(type, f, opts));
  };
  const activate = (entry: Entry) => {
    if (entry === active) return;
    active?.el.classList.remove("tk-active");
    active = entry;
    entry.el.classList.add("tk-active");
    const fe = document.activeElement;
    if (!fe || fe === document.body || fe.classList?.contains("tk-input")) entry.typer.focus();
  };
  const pick = () => {
    const mid = innerHeight / 2;
    let best = entries[0]!, bestD = Infinity;
    for (const e of entries) {
      const r = e.el.getBoundingClientRect();
      if (r.bottom < 0 || r.top > innerHeight) continue;
      const d = Math.abs((r.top + r.bottom) / 2 - mid);
      if (d < bestD) { bestD = d; best = e; }
    }
    if (held) {
      if (best !== held) return;
      held = null;
      clearTimeout(holdTimer);
    }
    activate(best);
  };
  on(window, "scroll", () => requestAnimationFrame(pick), { passive: true });
  const release = () => {
    if (!held) return;
    held = null;
    clearTimeout(holdTimer);
    pick();
  };
  on(window, "scrollend", release);
  on(window, "wheel", release, { passive: true });
  on(window, "touchstart", release, { passive: true });
  on(window, "resize", pick);
  on(document, "keydown", (ev: KeyboardEvent) => {
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
  for (const e of entries) on(e.typer.ta, "focus", () => activate(e));
  pick();
  entries[0]!.typer.focus();
  return {
    focus() { active?.typer.focus(); },
    hold(entry: Entry) {
      held = entry;
      clearTimeout(holdTimer);
      holdTimer = setTimeout(release, 1000);
    },
    destroy() {
      clearTimeout(holdTimer);
      for (const f of off) f();
      active?.el.classList.remove("tk-active");
    },
  };
}
