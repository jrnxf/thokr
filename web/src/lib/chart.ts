// The results chart of thokr's ui.rs, ported line by line: wpm_scale,
// nice_step, Plot and its axes, and ratatui's braille Canvas, so the page can
// draw the chart at its own size. web/test/snapshots.test.mjs checks it
// against thokr's insta snapshots on every change.

// ---------------------------------------------------------------- thok.rs

/** thok.rs calc_results. `keys` are [{ correct, at }], `at` in seconds from
 *  the start of the test; `elapsed` is how long the test ran. */
export type Key = { correct: boolean; at: number };
export type Coord = [number, number];
export type Results = { wpmCoords: Coord[]; elapsedSecs: number; stdDev: number; wpm: number; accuracy: number };

export function calcResults(keys: Key[], elapsed: number): Results {
  const whole = Math.floor(elapsed);
  // each correct key counts towards the whole second it fell in, or the
  // unfinished last second, plotted at the moment the test ended
  const seconds = keys
    .filter((k) => k.correct)
    .map(({ at }) => (at === 0 ? 1 : Math.ceil(at) <= whole ? Math.ceil(at) : elapsed))
    .sort((a, b) => a - b);
  const perSec: Coord[] = [];
  for (const s of seconds) {
    const last = perSec[perSec.length - 1];
    if (last && last[0] === s) last[1] += 1;
    else perSec.push([s, 1]);
  }
  // the last second is cut short, so it would skew the deviation
  const wholeCounts = perSec.slice(0, -1).map(([, c]) => c);
  let sum = 0;
  const wpmCoords = perSec.map(([s, c]): Coord => {
    sum += c;
    return [s, ((60 / s) * sum) / 5];
  });
  const last = wpmCoords[wpmCoords.length - 1];
  return {
    wpmCoords,
    elapsedSecs: elapsed,
    stdDev: stdDev(wholeCounts) ?? 0,
    wpm: last ? Math.ceil(last[1]) : 0,
    accuracy: keys.length ? Math.round((seconds.length / keys.length) * 100) : 0,
  };
}

/** util.rs std_dev: the population standard deviation, null for no data. */
export function stdDev(data: number[]): number | null {
  if (!data.length) return null;
  const mean = data.reduce((a, b) => a + b, 0) / data.length;
  return Math.sqrt(data.reduce((a, x) => a + (mean - x) ** 2, 0) / data.length);
}

// ---------------------------------------------------------------- ui.rs chart

const SEC = "sec";
const fmt = (v: number) => String(v); // Rust's {} for f64 prints 60 and 1.5 the same way

function niceStep(span: number, maxIntervals: number) {
  const raw = Math.max(span / Math.max(maxIntervals, 1), 1);
  const mag = 10 ** Math.floor(Math.log10(raw));
  for (const m of [1, 2, 5, 10]) if (m * mag >= raw - 1e-9) return m * mag;
  return raw;
}

type Scale = { lo: number; hi: number; step: number };

function wpmScale(values: number[], rows: number): Scale {
  let min = Infinity, max = -Infinity;
  for (const v of values) { min = Math.min(min, v); max = Math.max(max, v); }
  if (min > max) { min = 0; max = 10; }
  if (max - min < 10) {
    min = Math.max((min + max) / 2 - 5, 0);
    max = min + 10;
  }
  const intervals = Math.min(Math.max(Math.floor(rows / 4), 2), 6, Math.max(rows, 2) - 1);
  let step = niceStep(max - min, intervals);
  for (;;) {
    const lo = Math.floor(min / step) * step;
    const hi = Math.ceil(max / step) * step;
    if (Math.round((hi - lo) / step) <= intervals) return { lo, hi, step };
    step = niceStep(step * 1.5, 1);
  }
}
const ticksOf = (s: Scale) => {
  const count = Math.round((s.hi - s.lo) / s.step);
  return Array.from({ length: count + 1 }, (_, i) => s.lo + i * s.step);
};
const canvasCell = (offset: number, span: number, cells: number) =>
  Math.floor(Math.round((offset * (cells * 2 - 1)) / span) / 2);

/** A grid of `cols` x `rows` cells; each is {ch, k} with k one of
 *  "" (blank), "label", "rule", "line", "sec". Braille cells also carry
 *  `dots`, the 8-bit dot mask. */
export type Cell = { ch: string; k: string; dots?: number };
export type Grid = Cell[][];
type Area = { x: number; y: number; w: number; h: number };

export function makeGrid(cols: number, rows: number): Grid {
  return Array.from({ length: rows }, () =>
    Array.from({ length: cols }, () => ({ ch: " ", k: "" })),
  );
}
const put = (g: Grid, x: number, y: number, text: string, k: string) => {
  if (!g[y]) return;
  [...text].forEach((ch, i) => { if (g[y]![x + i]) g[y]![x + i] = { ch, k }; });
};

/** The chart part of the results screen (ui.rs Plot), drawn into `g` at
 *  the rectangle {x, y, w, h}. Returns the plot geometry. */
export function drawChart(g: Grid, coords: Coord[], area: Area) {
  const y = wpmScale(coords.map((c) => c[1]), Math.max(area.h - 1, 0));
  let gutter = ticksOf(y).map((v) => fmt(v).length).reduce((a, b) => Math.max(a, b), SEC.length) + 1;
  const first = coords.length ? coords[0]![0] : 0;
  const last = coords.length ? coords[coords.length - 1]![0] : 1;
  const [x0, x1]: Coord = last > first ? [Math.min(first, 1), last] : [0, Math.max(last * 2, 1)];
  gutter = Math.min(gutter, area.w);
  const P = { x: area.x + gutter, y: area.y, w: area.w - gutter, h: Math.max(area.h - 1, 0) };
  const plot = { area: P, gutter, y, x0, x1 };
  const col = (s: number) => P.x + canvasCell(s - x0, x1 - x0, P.w);
  const row = (w: number) => P.y + canvasCell(y.hi - w, y.hi - y.lo, P.h);
  if (!P.w || !P.h) return plot;

  // the line: ratatui's Canvas with Marker::Braille, 2x4 dots to a cell
  const resX = P.w * 2, resY = P.h * 4;
  const dots = new Map<number, number>();
  const point = (wx: number, wy: number): Coord | null => {
    if (wx < x0 || wx > x1 || wy < y.lo || wy > y.hi) return null;
    return [
      Math.round(((wx - x0) * (resX - 1)) / (x1 - x0)),
      Math.round(((y.hi - wy) * (resY - 1)) / (y.hi - y.lo)),
    ];
  };
  const paint = (px: number, py: number) => {
    const cx = Math.floor(px / 2), cy = Math.floor(py / 4);
    const bit = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]][py % 4]![px % 2]!;
    const key = cy * 10000 + cx;
    dots.set(key, (dots.get(key) || 0) | bit);
  };
  const line = (ax: number, ay: number, bx: number, by: number) => {
    // clip to the x bounds only: the y axis holds every value by design
    if (ax < x0) { ay = ay + ((by - ay) * (x0 - ax)) / (bx - ax); ax = x0; }
    if (bx > x1) { by = ay + ((by - ay) * (x1 - ax)) / (bx - ax); bx = x1; }
    const a = point(ax, ay), b = point(bx, by);
    if (a && b) bresenham(a[0], a[1], b[0], b[1], paint);
  };
  for (let i = 1; i < coords.length; i++) line(...coords[i - 1]!, ...coords[i]!);
  if (coords.length === 1) {
    const [s, w] = coords[0]!;
    const half = (1.5 * (x1 - x0)) / P.w;
    line(s - half, w, s + half, w);
  }
  for (const [key, mask] of dots) {
    const cy = Math.floor(key / 10000), cx = key % 10000;
    g[P.y + cy]![P.x + cx] = { ch: String.fromCharCode(0x2800 + mask), k: "line", dots: mask };
  }

  // axes: wpm labels with a faint rule, seconds labels underneath
  for (const w of ticksOf(y).reverse().slice(0, P.h)) {
    const r = row(w);
    const label = fmt(w);
    put(g, P.x - 1 - label.length, r, label, "label");
    for (let x = P.x; x < P.x + P.w; x++) if (g[r]![x]!.ch === " ") g[r]![x] = { ch: "─", k: "rule" };
  }
  for (const [x, label] of secondsLabels(plot, col)) put(g, x, P.y + P.h, label, "label");
  put(g, P.x - gutter, P.y + P.h, SEC, "sec");
  return plot;
}

function secondsLabels({ area: P, x0, x1 }: { area: Area; x0: number; x1: number }, col: (s: number) => number) {
  const span = x1 - x0;
  const intervals = Math.min(Math.max(Math.floor(P.w / 9), 2), 10);
  let step;
  if (span < 2) step = 0.5;
  else {
    step = niceStep(span, intervals);
    if (step > x1) step = niceStep(span, intervals + 1);
  }
  let secs = Math.ceil(x0 / step) * step;
  if (secs === 0) secs = step;
  const labels: [number, string][] = [];
  let nextFree = P.x;
  while (secs <= x1 + 1e-9) {
    const label = fmt(secs);
    const len = label.length;
    if (len <= P.w) {
      const start = Math.min(Math.max(col(secs) - Math.floor(len / 2), 0), P.x + P.w - len);
      if (start >= nextFree) {
        labels.push([start, label]);
        nextFree = start + len + 1;
      }
    }
    secs += step;
  }
  return labels;
}

function bresenham(x1: number, y1: number, x2: number, y2: number, f: (x: number, y: number) => void) {
  const dx = Math.abs(x2 - x1), dy = Math.abs(y2 - y1);
  if (dx === 0) { for (let y = Math.min(y1, y2); y <= Math.max(y1, y2); y++) f(x1, y); return; }
  if (dy === 0) { for (let x = Math.min(x1, x2); x <= Math.max(x1, x2); x++) f(x, y1); return; }
  if (dy < dx) {
    if (x1 > x2) [x1, y1, x2, y2] = [x2, y2, x1, y1];
    let d = 2 * dy - dx, y = y1;
    for (let x = x1; x <= x2; x++) {
      f(x, y);
      if (d > 0) { y += y1 > y2 ? -1 : 1; d -= 2 * dx; }
      d += 2 * dy;
    }
  } else {
    if (y1 > y2) [x1, y1, x2, y2] = [x2, y2, x1, y1];
    let d = 2 * dx - dy, x = x1;
    for (let y = y1; y <= y2; y++) {
      f(x, y);
      if (d > 0) { x += x1 > x2 ? -1 : 1; d -= 2 * dy; }
      d += 2 * dx;
    }
  }
}

/** The stats line under the chart, as ui.rs prints it. */
export const statsLine = (t: Pick<Results, "wpm" | "accuracy" | "stdDev">) => `${t.wpm} wpm   ${t.accuracy}% acc   ${t.stdDev.toFixed(2)} sd`;
export const LEGEND = "(r)etry / (n)ew / (esc)ape";

/** The whole results screen (ui.rs render_results) in a cols x rows grid. */
export function resultsGrid(thok: Pick<Results, "wpmCoords" | "wpm" | "accuracy" | "stdDev">, cols: number, rows: number): Grid {
  const g = makeGrid(cols, rows);
  const mx = Math.min(5, Math.floor(cols / 2)), my = Math.min(2, Math.floor(rows / 2));
  const w = cols - 2 * mx, h = rows - 2 * my;
  const chartH = Math.max(h - 3, 1);
  drawChart(g, thok.wpmCoords, { x: mx, y: my, w, h: chartH });
  const stats = statsLine(thok);
  // a Paragraph cuts its text off at the edge of its area
  put(g, mx + Math.max(Math.floor((w - stats.length) / 2), 0), my + chartH, stats.slice(0, w), "stats");
  put(g, mx, my + chartH + 2, LEGEND.slice(0, w), "legend");
  return g;
}

/** A chart alone, sized to cols x rows. */
export function chartGrid(coords: Coord[], cols: number, rows: number): Grid {
  const g = makeGrid(cols, rows);
  drawChart(g, coords, { x: 0, y: 0, w: cols, h: rows });
  return g;
}

/** Draws a grid as SVG. Braille cells become their dots, so the chart does
 *  not depend on a font that has the braille block. */
export function gridSVG(g: Grid, { cw = 10, ch = 20, dot = 0.17, fs = null as number | null } = {}) {
  const rows = g.length, cols = g[0]?.length || 0;
  const W = cols * cw, H = rows * ch;
  fs = fs || ch * 0.62;
  const out = [`<svg class="tk-grid" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" xmlns="http://www.w3.org/2000/svg" role="img">`];
  const r = Math.min(cw / 2, ch / 4) * dot * 2.2;
  for (let y = 0; y < rows; y++) {
    let x = 0;
    while (x < cols) {
      const c = g[y]![x]!;
      if (c.k === "line") {
        for (let i = 0; i < 8; i++) {
          if (!(c.dots! & (1 << i))) continue;
          const [dx, dy] = [[0, 0], [0, 1], [0, 2], [1, 0], [1, 1], [1, 2], [0, 3], [1, 3]][i]!;
          out.push(`<circle class="tk-dot" cx="${(x * cw + (dx + 0.5) * (cw / 2)).toFixed(2)}" cy="${(y * ch + (dy + 0.5) * (ch / 4)).toFixed(2)}" r="${r.toFixed(2)}"/>`);
        }
        x++;
      } else if (c.k === "rule") {
        let e = x;
        while (e < cols && g[y]![e]!.k === "rule") e++;
        out.push(`<line class="tk-rule" x1="${x * cw}" x2="${e * cw}" y1="${y * ch + ch / 2}" y2="${y * ch + ch / 2}"/>`);
        x = e;
      } else if (c.k) {
        let e = x, text = "";
        while (e < cols && g[y]![e]!.k === c.k) { text += g[y]![e]!.ch; e++; }
        out.push(`<text class="tk-${c.k}" x="${x * cw}" y="${y * ch + ch * 0.7}" font-size="${fs}" textLength="${(e - x) * cw}" lengthAdjust="spacingAndGlyphs" xml:space="preserve">${esc(text)}</text>`);
        x = e;
      } else x++;
    }
  }
  out.push("</svg>");
  return out.join("");
}
const esc = (s: string) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!);

