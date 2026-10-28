import type { EngineState } from "../lib/engine.ts";

/** A prompt from the engine's state, one span per char: c correct, x
 *  incorrect, u untyped, cur where the next key goes, pace for the pace caret. */
export function Prompt({ state, ...rest }: { state: EngineState | null } & React.ComponentProps<"p">) {
  const chars = state ? Array.from(state.prompt) : [];
  return (
    <p {...rest} className={state && state.input.length === chars.length ? "p tk-end" : "p"}>
      {chars.map((ch, i) => {
        const o = state!.input[i];
        let cls = o ? (o === "c" ? "c" : "x") : "u";
        if (i === state!.input.length) cls += " cur";
        if (state!.pace === i) cls += " pace";
        // thokr shows a mistyped space as a dot
        return <span key={i} className={cls}>{o === "x" && ch === " " ? "·" : ch}</span>;
      })}
    </p>
  );
}
