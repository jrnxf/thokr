# thokr.jrnxf.co

The website: one page where every line is a thokr prompt you can type. The tests are run by
thokr itself: `engine/` compiles thokr's own `src/` (the test loop, the screen and the word lists)
to WebAssembly, and the page sends it keys, as `main.rs` does in a terminal.

- `src/`: the page, in TypeScript and React, built with Vite; `src/lib/` holds the engine's
  WebAssembly shim, the typing loop and the chart. `public/` has DM Mono.
- `engine/`: the WebAssembly build of thokr, with a small C ABI in place of `main.rs`.
- `build.sh`: builds everything into `dist/`. It installs Rust when it is missing.
- `test/`: checks the engine and `src/lib/chart.ts` against thokr's insta snapshots, and types
  through the page in Chromium.

```sh
cd web
npm ci
./build.sh
npx playwright install chromium
npm test
npx wrangler dev   # serves dist on localhost
npm run typecheck  # tsc, which `./build.sh` also runs
```

It deploys to thokr.jrnxf.co with Cloudflare Workers Builds: the build command is `./build.sh`, the
deploy command `npx wrangler deploy`, and the root directory `web`. `wrangler.jsonc` holds the rest.
