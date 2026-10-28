import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// the version in the footer is thokr's own, from the Cargo.toml above web/
const cargo = readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
const version = cargo.match(/^version = "(.*)"$/m)?.[1] ?? "";

export default defineConfig({
  plugins: [react()],
  define: { __THOKR_VERSION__: JSON.stringify(version) },
  // the engine is a .wasm that build.sh drops in wasm/; it is always a file of its own
  build: { assetsInlineLimit: 0 },
});
