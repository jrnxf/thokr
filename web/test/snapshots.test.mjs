// Checks the website against thokr's own insta snapshots (src/snapshots), so
// a change to ui.rs that the page does not follow fails here:
// - every running-screen snapshot is drawn again by the WebAssembly engine
//   that build.sh compiles from src/, typed key by key;
// - every results-screen snapshot is drawn again by kit/chart.js, the page's
//   JavaScript port of the chart.
// The fixtures are read from the tests in src/ui.rs. Run after build.sh.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { calcResults, resultsGrid } from "../public/kit/chart.js";
import { loadEngine } from "../dist/kit/engine.js";

const root = new URL("../../", import.meta.url);
const ui = await readFile(new URL("src/ui.rs", root), "utf8");
const snapshotDir = new URL("src/snapshots/", root);
const names = (await readdir(fileURLToPath(snapshotDir)))
  .filter((f) => f.startsWith("thokr__ui__tests__") && f.endsWith(".snap"))
  .map((f) => f.slice("thokr__ui__tests__".length, -".snap".length));

/** A snapshot as { rows, cursor }: the screen's lines and the cursor line. */
async function snapshot(name, height) {
  const text = await readFile(new URL(`thokr__ui__tests__${name}.snap`, snapshotDir), "utf8");
  const body = text.split("\n---\n").slice(1).join("\n---\n").split("\n");
  return { rows: body.slice(0, height), cursor: body.find((l) => l.startsWith("cursor = ")) };
}

const consts = Object.fromEntries([...ui.matchAll(/const (\w+): &str = "([^"]*)";/g)].map((m) => [m[1], m[2]]));
const str = (s) => (s.startsWith('"') ? JSON.parse(s) : consts[s]);
/** The body of `fn name()` in ui.rs's tests. */
function body(name) {
  const at = ui.indexOf(`fn ${name}()`);
  assert.notEqual(at, -1, `ui.rs has no fn ${name}()`);
  return ui.slice(at, ui.indexOf("\n    }\n", at));
}
const ARG = String.raw`("[^"]*"|[A-Z_]+)`;

for (const name of names.filter((n) => n.startsWith("running_"))) {
  test(`${name}, drawn by the WebAssembly engine`, async () => {
    const fn = body(name);
    const typed = fn.match(new RegExp(String.raw`Thok::typed\(\s*${ARG},\s*${ARG},\s*None,\s*&\[\],?\s*\)`));
    const size = fn.match(/draw\(&thok, (\d+), (\d+)\)/);
    assert.ok(typed && size, `${name}: no Thok::typed(prompt, typed, None, &[]) and draw(&thok, w, h) to replay`);
    const [w, h] = [+size[1], +size[2]];
    const engine = await loadEngine();
    engine.config({ prompt: str(typed[1]) });
    for (const c of str(typed[2])) engine.key(c);
    const frame = engine.frame(w, h);
    const want = await snapshot(name, h);
    assert.deepEqual(frame.rows.map((r) => r.map((c) => (c ? c[0] : " ")).join("")), want.rows);
    const cursor = frame.cursor ? `cursor = Some(Position { x: ${frame.cursor[0]}, y: ${frame.cursor[1]} })` : "cursor = None";
    assert.equal(cursor, want.cursor);
  });
}

// finished() in ui.rs: a prompt typed with a key at each offset, 60 s ago
const finished = body("finished");
const fixture = {
  prompt: str(finished.match(new RegExp(`let prompt = ${ARG};`))[1]),
  typed: str(finished.match(new RegExp(`Thok::typed\\(prompt, ${ARG}`))[1]),
  offsets: finished.match(/\[([\d,\s]+)\]/)[1].split(",").map(Number).filter((n) => !Number.isNaN(n)),
};

for (const name of names.filter((n) => n.startsWith("results_"))) {
  test(`${name}, drawn by kit/chart.js`, async () => {
    const fn = body(name);
    const size = fn.match(/draw\(&(\w+(?:\(\))?), (\d+), (\d+)\)/);
    assert.ok(size, `${name}: no draw(&thok, w, h)`);
    const [w, h] = [+size[2], +size[3]];
    let thok;
    if (size[1] === "finished()") {
      const chars = [...fixture.prompt];
      const keys = [...fixture.typed].map((c, i) => ({ correct: c === chars[i], at: fixture.offsets[i] / 1000 }));
      thok = calcResults(keys, 60);
    } else if (/Thok::new\(String::new\(\), 0, None\)/.test(fn)) {
      // a test that never started: calc_results never ran
      thok = { wpmCoords: [], wpm: 0, accuracy: 0, stdDev: 0 };
    } else {
      assert.fail(`${name}: teach this test its fixture`);
    }
    const want = await snapshot(name, h);
    assert.deepEqual(resultsGrid(thok, w, h).map((r) => r.map((c) => c.ch).join("")), want.rows);
  });
}

test("every snapshot of ui.rs is checked", () => {
  const checked = names.filter((n) => n.startsWith("running_") || n.startsWith("results_"));
  assert.deepEqual(checked, names, "a new kind of snapshot needs a check here");
  assert.ok(names.length >= 9);
});
