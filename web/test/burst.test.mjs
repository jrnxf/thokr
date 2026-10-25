// Typing through the page in a real browser, as fast as keys go: a block that
// is finished must move on to the next one, with the keys that keep coming,
// and never reset. Mashing keys used to restart blocks, because thokr reads r
// and n after a test as "again" and "new words". Run after build.sh, with
// Chromium installed by `npx playwright install chromium`.
import { test, before, after } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname } from "node:path";
import { chromium } from "playwright";

const dist = new URL("../dist/", import.meta.url);
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".woff2": "font/woff2" };
let server, browser, url;

before(async () => {
  server = createServer(async (req, res) => {
    const path = new URL(req.url, "http://x").pathname.replace(/^\/$/, "/index.html");
    try {
      const body = await readFile(new URL("." + path, dist));
      res.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" }).end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  url = `http://127.0.0.1:${server.address().port}/`;
  browser = await chromium.launch();
});
after(async () => {
  await browser?.close();
  server?.close();
});

/** A page with its engines loaded and the first prompt on screen. */
async function open(options = {}) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, ...options });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e));
  await page.goto(url);
  await page.waitForFunction(() => document.querySelectorAll("#prompt span").length > 0 && document.querySelector(".tk-active"));
  return { page, errors };
}
const SECTIONS = ".line";
const say = (page, i) => page.$$eval(SECTIONS, (els, i) => els[i].querySelector("[data-say]").dataset.say, i);
/** Which block has the keyboard: "hero" or the index of a section. */
const active = (page) =>
  page.evaluate(() => {
    const a = document.querySelector(".tk-active");
    const focused = a.contains(document.activeElement);
    return { block: a.id === "hero" ? "hero" : [...document.querySelectorAll(".line")].indexOf(a), focused };
  });

test("mashing keys past the end of a sentence moves on and keeps typing", async () => {
  const { page, errors } = await open();
  await page.$$eval(SECTIONS, (els) => els[0].scrollIntoView({ block: "center" }));
  await page.waitForFunction(() => document.querySelectorAll(".line")[0].classList.contains("tk-active"));
  // the sentence, then a burst of keys with thokr's r and n among them, no pause
  await page.keyboard.type((await say(page, 0)) + "rnrnwhen", { delay: 0 });
  await page.waitForTimeout(400);

  const first = await page.$$eval(SECTIONS, (els) => els[0].querySelector(".after").textContent);
  assert.match(first, /wpm/, "the first sentence was finished, not reset");
  assert.deepEqual(await active(page), { block: 1, focused: true }, "the next sentence has the keyboard");
  const typed = await page.$$eval(SECTIONS, (els) => els[1].querySelectorAll(".p .c, .p .x").length);
  assert.equal(typed, 8, "the 8 keys after the first sentence went on into the next");
  assert.deepEqual(errors, []);
  await page.close();
});

test("mashing past the end of the test at the top moves on to the first sentence", async () => {
  const { page, errors } = await open();
  const prompt = await page.$eval("#prompt", (el) => el.textContent);
  await page.keyboard.type(prompt + "nrnr", { delay: 0 });
  await page.waitForTimeout(400);
  assert.equal(await page.$eval("#res", (el) => el.hidden), false, "the test at the top shows its results");
  assert.deepEqual(await active(page), { block: 0, focused: true });
  assert.equal(await page.$$eval(SECTIONS, (els) => els[0].querySelectorAll(".p .c, .p .x").length), 4);
  assert.deepEqual(errors, []);
  await page.close();
});

test("typing through every block with no clicks ends on the install footer", async () => {
  const { page, errors } = await open({ reducedMotion: "reduce" });
  await page.keyboard.type(await page.$eval("#prompt", (el) => el.textContent), { delay: 0 });
  const count = await page.$$eval(SECTIONS, (els) => els.length);
  for (let i = 0; i < count; i++) {
    await page.waitForFunction((i) => document.querySelectorAll(".line")[i].classList.contains("tk-active"), i);
    assert.equal((await active(page)).focused, true, `block ${i} has the keyboard`);
    await page.keyboard.type(await say(page, i), { delay: 0 });
  }
  assert.equal(await page.$eval("#install", (el) => el.classList.contains("done")), true, "install grows after the last key");
  assert.deepEqual(errors, []);
  await page.close();
});

test("keys typed while the page scrolls to the next sentence stay with it", async () => {
  const { page, errors } = await open();
  await page.$$eval(SECTIONS, (els) => els[0].scrollIntoView({ block: "center" }));
  await page.waitForFunction(() => document.querySelectorAll(".line")[0].classList.contains("tk-active"));
  const next = await say(page, 1);
  const wrong = next[0] === "x" ? "z" : "x";
  await page.keyboard.type((await say(page, 0)) + wrong, { delay: 0 });
  // mid-scroll, while the finished sentence is still nearest the middle
  await page.waitForTimeout(120);
  await page.keyboard.press("Backspace");
  await page.keyboard.press("ArrowLeft");
  await page.waitForTimeout(80);
  await page.keyboard.type(next.slice(0, 3), { delay: 0 });
  await page.waitForTimeout(800);

  assert.match(await page.$$eval(SECTIONS, (els) => els[0].querySelector(".after").textContent), /wpm/, "the first sentence stays finished");
  assert.deepEqual(await active(page), { block: 1, focused: true });
  const marks = await page.$$eval(SECTIONS, (els) => [...els[1].querySelectorAll(".p .c, .p .x")].map((s) => s.className));
  assert.deepEqual(marks, ["c", "c", "c"], "the Backspace undid the mistake in the sentence being typed");
  assert.deepEqual(errors, []);
  await page.close();
});
