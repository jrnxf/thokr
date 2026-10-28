import { autoType, type Entry } from "./autotype.ts";
import { loadEngine, type Config, type Engine, type EngineState } from "./engine.ts";
import { Typer } from "./typer.ts";
import wasmUrl from "../../wasm/thokr.wasm?url";

export type Block = { root: HTMLElement; ta: HTMLTextAreaElement; focusEl?: HTMLElement };

export type Handlers = {
  /** every state of the test at the top (the hero), and of each sentence below it */
  render: (block: number, state: EngineState) => void;
  onFinish: (block: number, state: EngineState) => void;
};

const reduceMotion = () => matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Every block of the page is a test of its own, each with its own copy of
 * thokr; block 0 is the hero, the sentences follow in page order.
 * When a block is finished, the next one comes to the middle of the window
 * and takes the keyboard, so typing just continues: after a moment, or at
 * once when the next key comes first. After the last block, install takes
 * the stage. An arrow key in the meantime keeps you where you are.
 */
export async function startSession(
  blocks: Block[],
  configs: Config[],
  install: HTMLElement,
  h: Handlers,
) {
  const engines: Engine[] = [];
  for (let i = 0; i < blocks.length; i++) engines.push(await loadEngine(new URL(wasmUrl, location.href)));

  const entries: Entry[] = [];
  const pending = new Map<number, ReturnType<typeof setTimeout>>();
  let auto: ReturnType<typeof autoType>;
  const moveOn = (i: number) => {
    clearTimeout(pending.get(i));
    pending.delete(i);
    const from = entries[i]!, next = entries[i + 1];
    if (!from.typer.e.state().finished) return null;
    const behavior = reduceMotion() ? "auto" : "smooth";
    if (!next) {
      install.scrollIntoView({ behavior, block: "end" });
      return null;
    }
    auto.hold(next);
    next.el.scrollIntoView({ behavior, block: "center" });
    next.typer.focus();
    return next;
  };
  const advance = (i: number, wait: number) => {
    clearTimeout(pending.get(i));
    pending.set(i, setTimeout(() => moveOn(i), wait));
  };
  // a key typed after a block is finished goes on into the next block
  const carryOn = (i: number) => (key: string) => moveOn(i)?.typer.press(key);

  blocks.forEach((b, i) => {
    const typer = new Typer(b.root, b.ta, engines[i]!, {
      config: configs[i],
      focusEl: b.focusEl,
      render: (s) => h.render(i, s),
      onFinish: (s) => { h.onFinish(i, s); advance(i, i === 0 ? 1400 : 700); },
      afterFinish: carryOn(i),
    });
    entries.push({ typer, el: b.root });
  });
  auto = autoType(entries);
  return {
    hero: entries[0]!.typer,
    focus: () => auto.focus(),
    destroy() {
      for (const t of pending.values()) clearTimeout(t);
      auto.destroy();
      for (const e of entries) e.typer.destroy();
    },
  };
}
