import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Prompt } from "./components/Prompt.tsx";
import { chartGrid, gridSVG } from "./lib/chart.ts";
import type { EngineState } from "./lib/engine.ts";
import { cmdFor, copy, initialTheme, type Theme } from "./lib/page.ts";
import { startSession } from "./lib/session.ts";

const LINES = [
  {
    id: "what",
    say: "thokr is a typing test that runs in your terminal",
    note: <>15 of the 200 most common English words. Mistakes show in red; backspace fixes them.</>,
  },
  {
    say: "when you finish, it plots your speed for each second",
    note: <>Words per minute against seconds, then accuracy and how steady you were.</>,
  },
  {
    say: "each test adds a row to a log file",
    note: <><code>log.csv</code> in thokr's config folder, one row per test.</>,
  },
  {
    say: "one flag changes the test",
    note: (
      <>
        <code>-w 50</code> · <code>-s 30</code> · <code>--pace 60</code> · <code>{'-p "text"'}</code> · <code>-f 2</code> · <code>-l english1k</code>
      </>
    ),
  },
];

const WORDS = [[15, "15"], [30, "30"], [50, "50"]] as const;
const LIMITS = [[0, "none"], [30, "30s"]] as const;
const PACES = [[0, "off"], [60, "60"]] as const;

type Opts = { words: number; secs: number; pace: number };
type Session = Awaited<ReturnType<typeof startSession>>;

const stats = (s: EngineState) => `${s.wpm} wpm   ${s.accuracy}% acc   ${s.sd.toFixed(2)} sd`;

function CopyButton({ text, id }: { text: string; id: string }) {
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1400);
    return () => clearTimeout(t);
  }, [copied]);
  return (
    <p>
      <code id={id}>{text}</code>
      <button type="button" onClick={async () => { if (await copy(text)) setCopied(true); }}>{copied ? "copied" : "copy"}</button>
    </p>
  );
}

export function App() {
  const [theme, setTheme] = useState<Theme>(initialTheme);
  useLayoutEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);

  useEffect(() => {
    // DM Mono is preloaded from the head; show the page once both weights are in
    const show = () => document.getElementById("root")?.classList.add("ready");
    const timeout = setTimeout(show, 3000);
    Promise.all([document.fonts.load('400 1em "DM Mono"'), document.fonts.load('500 1em "DM Mono"')])
      .catch(() => {})
      .then(() => { clearTimeout(timeout); show(); });
    return () => clearTimeout(timeout);
  }, []);

  const [opts, setOpts] = useState<Opts>({ words: 15, secs: 0, pace: 0 });
  const [hero, setHero] = useState<EngineState | null>(null);
  const [results, setResults] = useState<{ stats: string; chart: string } | null>(null);
  const [lines, setLines] = useState<(EngineState | null)[]>(LINES.map(() => null));
  const [done, setDone] = useState(false);

  const heroEl = useRef<HTMLDivElement>(null);
  const heroTa = useRef<HTMLTextAreaElement>(null);
  const lineEls = useRef<(HTMLElement | null)[]>([]);
  const lineTas = useRef<(HTMLTextAreaElement | null)[]>([]);
  const promptEls = useRef<(HTMLElement | null)[]>([]);
  const installEl = useRef<HTMLElement>(null);
  const session = useRef<Session | null>(null);

  useEffect(() => {
    let gone = false, started: Session | undefined;
    const onResize = () => started?.hero.after();
    startSession(
      [
        { root: heroEl.current!, ta: heroTa.current! },
        ...LINES.map((_, i) => ({ root: lineEls.current[i]!, ta: lineTas.current[i]!, focusEl: promptEls.current[i]! })),
      ],
      [{ words: 15, secs: 0, pace: 0 }, ...LINES.map((l) => ({ prompt: l.say }))],
      installEl.current!,
      {
        render(block, s) {
          if (block === 0) setHero(s);
          else setLines((all) => all.map((x, i) => (i === block - 1 ? s : x)));
        },
        onFinish(block, s) {
          if (block === 0) {
            const narrow = heroEl.current!.clientWidth < 600;
            setResults({
              stats: stats(s),
              chart: gridSVG(chartGrid(s.coords, narrow ? 34 : 70, narrow ? 10 : 12), { cw: 10, ch: 20 }),
            });
          } else if (block === LINES.length) setDone(true);
        },
      },
    ).then((s) => {
      if (gone) return s.destroy();
      started = session.current = s;
    });
    addEventListener("resize", onResize);
    return () => {
      gone = true;
      removeEventListener("resize", onResize);
      started?.destroy();
      session.current = null;
    };
  }, []);

  function pick(change: Partial<Opts>) {
    const next = { ...opts, ...change };
    if (change.secs !== undefined) next.words = next.secs ? 100 : 15;
    setOpts(next);
    session.current?.hero.set({ ...next });
    session.current?.hero.focus();
  }

  const finished = hero?.finished ?? false;
  const timer = hero && hero.remaining != null && !hero.finished ? hero.remaining.toFixed(1) : "";

  return (
    <>
      <header>
        <div className="wrap">
          <a className="mark" href="#top">thokr</a>
          <nav aria-label="Sections">
            <a className="hide" href="#install">install</a>
            <a href="https://github.com/jrnxf/thokr">source</a>
            <button id="tog" type="button" aria-pressed={theme === "dark"} onClick={() => setTheme(theme === "dark" ? "light" : "dark")}>theme</button>
          </nav>
        </div>
      </header>

      <main id="top">
        <div className="wrap">
          <div className={hero ? "hero" : "hero booting"} id="hero" ref={heroEl}>
            <p className="lead"><b>Start typing.</b> This is thokr, a typing test for the terminal, running in the page.</p>
            <div className="run" id="run" hidden={finished}><Prompt id="prompt" state={hero} /></div>
            <div className="res" id="res" hidden={!finished}>
              <p className="stats" id="stats">
                {results?.stats.split(/(wpm|% acc|sd)/).map((part, i) => (i % 2 ? <i key={i}>{part}</i> : part))}
              </p>
              <div className="chart" id="chart" dangerouslySetInnerHTML={{ __html: results?.chart ?? "" }} />
              <p className="again">left arrow: the same words · right arrow: new words</p>
            </div>
            <div className="meta" id="opts">
              <span className="timer" id="timer">{timer}</span>
              <span>words {WORDS.map(([n, label]) => (
                <button key={n} aria-pressed={opts.words === n} onClick={() => pick({ words: n })}>{label}</button>
              ))}</span>
              <span>limit {LIMITS.map(([n, label]) => (
                <button key={n} aria-pressed={opts.secs === n} onClick={() => pick({ secs: n })}>{label}</button>
              ))}</span>
              <span>pace {PACES.map(([n, label]) => (
                <button key={n} aria-pressed={opts.pace === n} onClick={() => pick({ pace: n })}>{label}</button>
              ))}</span>
              <span className="cmd" id="cmd">{cmdFor(opts)}</span>
            </div>
            <textarea ref={heroTa} {...INPUT} />
          </div>
        </div>

        <div className="wrap">
          {LINES.map((line, i) => {
            const s = lines[i];
            return (
              <section className="line" id={line.id} key={line.say} ref={(el) => { lineEls.current[i] = el; }}>
                <Prompt state={s} data-say={line.say} ref={(el) => { promptEls.current[i] = el; }} />
                <div className="after">
                  {s?.finished
                    ? <><b>{s.wpm} wpm</b>{` · ${s.accuracy}% acc · ${s.sd.toFixed(2)} sd · left arrow: again`}</>
                    : `thokr -p "${line.say}"`}
                </div>
                <p className="note">{line.note}</p>
                <textarea ref={(el) => { lineTas.current[i] = el; }} {...INPUT} />
              </section>
            );
          })}
        </div>
      </main>

      <footer id="install" className={done ? "done" : undefined} ref={installEl}>
        <div className="wrap inst">
          <h2>install</h2>
          <div className="cmds">
            <CopyButton id="i1" text="brew install jrnxf/tap/thokr" />
            <CopyButton id="i2" text="cargo install thokr" />
            <p><a href="https://github.com/jrnxf/thokr">source on GitHub</a></p>
          </div>
        </div>
        <div className="wrap base">
          <span>{`thokr ${__THOKR_VERSION__} · MIT · `}<a href="https://github.com/jrnxf/thokr">github.com/jrnxf/thokr</a></span>
          <span>DM Mono by Colophon, <a href="https://github.com/jrnxf/thokr/blob/main/web/THIRD_PARTY.md">SIL OFL</a></span>
          <span>also: <a href="https://okfs.jrnxf.co">okfs</a> · <a href="https://drip.jrnxf.co">drip</a></span>
        </div>
      </footer>
    </>
  );
}

/** The hidden textarea that takes a block's keys, so phone keyboards work. */
const INPUT = {
  className: "tk-input",
  name: "thokr-input",
  autoCapitalize: "off",
  autoComplete: "off",
  autoCorrect: "off",
  spellCheck: false,
  "aria-label": "Type the prompt here",
} as const;
