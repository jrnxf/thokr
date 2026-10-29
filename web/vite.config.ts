import { readFileSync } from "node:fs";
import { defineConfig, type HtmlTagDescriptor, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

// the version in the footer is thokr's own, from the Cargo.toml above web/
const cargo = readFileSync(new URL("../Cargo.toml", import.meta.url), "utf8");
const version = cargo.match(/^version = "(.*)"$/m)?.[1] ?? "";

/**
 * Starts the fonts, the engine and the stylesheet early: preload links in the
 * head, and the same as Link headers in _headers, which Cloudflare sends as
 * Early Hints. crossorigin is needed even same-origin, or a font is fetched twice.
 */
function preload(): Plugin {
  const all = (bundle: Record<string, unknown>) => Object.keys(bundle).filter((f) => f.startsWith("assets/"));
  const pick = (bundle: Record<string, unknown>) => {
    const files = all(bundle);
    const css = files.filter((f) => f.endsWith(".css"));
    const fonts = files.filter((f) => f.endsWith(".woff2"));
    const wasm = files.filter((f) => f.endsWith(".wasm"));
    return { css, fonts, wasm };
  };
  return {
    name: "thokr-preload",
    apply: "build",
    transformIndexHtml: {
      order: "post",
      handler(_html, ctx) {
        const { fonts, wasm } = pick(ctx.bundle ?? {});
        const tags: HtmlTagDescriptor[] = [
          ...fonts.map((f) => ({ tag: "link", attrs: { rel: "preload", href: `/${f}`, as: "font", type: "font/woff2", crossorigin: true }, injectTo: "head-prepend" as const })),
          ...wasm.map((f) => ({ tag: "link", attrs: { rel: "preload", href: `/${f}`, as: "fetch", type: "application/wasm", crossorigin: true }, injectTo: "head-prepend" as const })),
        ];
        return tags;
      },
    },
    generateBundle(_options, bundle) {
      const { css, fonts, wasm } = pick(bundle);
      const link = (f: string, as: string, extra = "") => `  Link: </${f}>; rel=preload; as=${as}${extra}`;
      const source = [
        "/",
        ...css.map((f) => link(f, "style")),
        ...fonts.map((f) => link(f, "font", "; type=font/woff2; crossorigin")),
        ...wasm.map((f) => link(f, "fetch", "; type=application/wasm; crossorigin")),
        "",
        "# everything under assets/ is named by its content, so it never changes under its name",
        "/assets/*",
        "  Cache-Control: public, max-age=31536000, immutable",
        "",
      ].join("\n");
      this.emitFile({ type: "asset", fileName: "_headers", source });
    },
  };
}

export default defineConfig({
  plugins: [react(), preload()],
  define: { __THOKR_VERSION__: JSON.stringify(version) },
  // the engine is a .wasm that build.sh drops in wasm/; it is always a file of its own
  build: { assetsInlineLimit: 0 },
});
