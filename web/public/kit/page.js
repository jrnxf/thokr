// Small helpers for the page.

/** Light or dark: ?theme=dark|light, then the system setting; the button flips it. */
export function theme(button) {
  const q = new URLSearchParams(location.search).get("theme");
  const root = document.documentElement;
  const set = (t) => {
    root.dataset.theme = t;
    button?.setAttribute("aria-pressed", String(t === "dark"));
  };
  set(q === "dark" || q === "light" ? q : matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
  button?.addEventListener("click", () => set(root.dataset.theme === "dark" ? "light" : "dark"));
}

/** The thokr command that runs the same test in a terminal. */
export function cmdFor({ words = 15, secs = 0, pace = 0 } = {}) {
  let c = "thokr";
  if (words !== 15) c += ` -w ${words}`;
  if (secs) c += ` -s ${secs}`;
  if (pace) c += ` --pace ${pace}`;
  return c;
}

/** Copies text; resolves true when the clipboard took it. */
export async function copy(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
