// Small helpers for the page.

export type Theme = "light" | "dark";

/** Light or dark: ?theme=dark|light, then the system setting. */
export function initialTheme(): Theme {
  const q = new URLSearchParams(location.search).get("theme");
  return q === "dark" || q === "light" ? q : matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
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
export async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
