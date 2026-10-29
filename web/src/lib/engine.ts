// thokr's own Rust (src/thok.rs, ui.rs, layout.rs, lang.rs), compiled to
// WebAssembly for wasm32-wasip1 by web/build.sh. This file is the WASI shim
// it needs (a clock, random bytes, stderr) and the calls that send it keys
// and ticks, as main.rs does in a terminal.
import type { Coord } from "./chart.ts";

export const KEY = { Backspace: 0x110001, ArrowLeft: 0x110002, ArrowRight: 0x110003 } as const;
const LANGS = { english: 0, english1k: 1, english10k: 2 } as const;

export type Config = {
  words?: number;
  sentences?: number;
  secs?: number;
  pace?: number;
  lang?: keyof typeof LANGS;
  prompt?: string;
};

/** {prompt, input ("c"/"x" per key), started, finished, pace, remaining, wpm, accuracy, sd, elapsed, coords} */
export type EngineState = {
  prompt: string;
  input: string;
  started: boolean;
  finished: boolean;
  pace: number | null;
  remaining: number | null;
  words: number;
  secs: number | null;
  wpm: number;
  accuracy: number;
  sd: number;
  elapsed: number;
  coords: Coord[];
};

export type Engine = Awaited<ReturnType<typeof loadEngine>>;

let compiled: Promise<WebAssembly.Module> | undefined;
async function compile(url: URL) {
  compiled ??= (async () => {
    if (url.protocol === "file:") {
      // the tests run the engine in node
      const node = "node:fs/promises";
      const { readFile } = await import(/* @vite-ignore */ node);
      return WebAssembly.compile(await readFile(url));
    }
    return WebAssembly.compileStreaming(fetch(url));
  })();
  return compiled;
}

/** One running copy of thokr. Several can run side by side. */
export async function loadEngine(url: URL) {
  let mem: WebAssembly.Memory;
  const view = () => new DataView(mem.buffer);
  const wasi = {
    clock_time_get(id: number, _precision: bigint, out: number) {
      const ns = id === 0 ? BigInt(Date.now()) * 1000000n : BigInt(Math.round(performance.now() * 1e6));
      view().setBigUint64(out, ns, true);
      return 0;
    },
    random_get(ptr: number, len: number) {
      crypto.getRandomValues(new Uint8Array(mem.buffer, ptr, len));
      return 0;
    },
    environ_sizes_get(count: number, size: number) {
      view().setUint32(count, 0, true);
      view().setUint32(size, 0, true);
      return 0;
    },
    environ_get() { return 0; },
    fd_write(_fd: number, iovs: number, n: number, written: number) {
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
    proc_exit(code: number): never { throw new Error(`thokr exited with ${code}`); },
  };
  const instance = await WebAssembly.instantiate(await compile(url), { wasi_snapshot_preview1: wasi });
  const x = instance.exports as Record<string, any>;
  mem = x.memory;
  const read = (len: number) => JSON.parse(new TextDecoder().decode(new Uint8Array(mem.buffer, x.tk_out(), len)));
  return {
    /** The flags, as thokr takes them; 0 or "" means not given. */
    config({ words = 15, sentences = 0, secs = 0, pace = 0, lang = "english", prompt = "" }: Config = {}) {
      const b = new TextEncoder().encode(prompt || "");
      if (b.length) new Uint8Array(mem.buffer, x.tk_inbox(b.length), b.length).set(b);
      x.tk_config(words, sentences || 0, secs || 0, pace || 0, LANGS[lang] ?? 0, b.length);
    },
    key(k: number | string) {
      x.tk_key(typeof k === "number" ? k : (KEY as Record<string, number>)[k] ?? k.codePointAt(0));
    },
    tick() { x.tk_tick(); },
    nextTickMs(): number { return x.tk_next_tick_ms(); },
    /** thokr's screen at w x h cells: {w, h, cursor, rows[y][x] = 0 | [symbol, fg, modifiers]} */
    frame(w: number, h: number): { w: number; h: number; cursor: [number, number] | null; rows: (0 | [string, ...unknown[]])[][] } {
      return read(x.tk_render(w, h));
    },
    state(): EngineState { return read(x.tk_state()); },
  };
}
