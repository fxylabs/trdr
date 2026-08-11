import { expect, test } from "vitest";

import { terminalAppearance } from "./theme";

/** The tokens as `design/ui-kit/tokens.v2.css` spells them, resolved. */
const TOKENS: Readonly<Record<string, string>> = {
    "--trdr-terminal-background": "#0d1116",
    "--trdr-terminal-foreground": "#b7c0c8",
    "--trdr-terminal-strong": "#ecf0f2",
    "--trdr-terminal-dim": "#687581",
    "--trdr-terminal-cursor": "#c8f24c",
    "--trdr-terminal-selection": "rgb(200 242 76 / 15%)",
    "--trdr-terminal-success": "#70bf9b",
    "--trdr-terminal-warning": "#d5a451",
    "--trdr-terminal-error": "#dc726b",
    "--trdr-terminal-info": "#72a8df",
    "--trdr-terminal-magenta": "#b99adf",
    "--trdr-terminal-cyan": "#70b9bd",
    "--trdr-font-mono": "ui-monospace, SFMono-Regular, Menlo, monospace",
    "--trdr-leading-terminal": "1.5"
};

function read(name: string): string
{
    return TOKENS[name] ?? "";
}

/**
 * The eight base ANSI hues the gallery's palette panel shows, each filled from
 * the token that names it. A colour written in this app rather than read from
 * the token block would show up here as a value the token block does not have.
 */
test("the base ANSI palette comes from the token block", () =>
{
    const { theme } = terminalAppearance(read);

    expect(theme["red"]).toBe(TOKENS["--trdr-terminal-error"]);
    expect(theme["green"]).toBe(TOKENS["--trdr-terminal-success"]);
    expect(theme["yellow"]).toBe(TOKENS["--trdr-terminal-warning"]);
    expect(theme["blue"]).toBe(TOKENS["--trdr-terminal-info"]);
    expect(theme["magenta"]).toBe(TOKENS["--trdr-terminal-magenta"]);
    expect(theme["cyan"]).toBe(TOKENS["--trdr-terminal-cyan"]);
    expect(theme["black"]).toBe(TOKENS["--trdr-terminal-background"]);
    expect(theme["white"]).toBe(TOKENS["--trdr-terminal-foreground"]);

    for (const value of Object.values(theme))
    {
        expect(Object.values(TOKENS)).toContain(value);
    }
});

test("the wrapper, the cursor and the selection come from the same place", () =>
{
    const { theme, fontFamily, lineHeight } = terminalAppearance(read);

    expect(theme["background"]).toBe(TOKENS["--trdr-terminal-background"]);
    expect(theme["cursor"]).toBe(TOKENS["--trdr-terminal-cursor"]);
    expect(theme["selectionBackground"]).toBe(TOKENS["--trdr-terminal-selection"]);
    expect(fontFamily).toBe(TOKENS["--trdr-font-mono"]);
    expect(lineHeight).toBe(1.5);
});

/**
 * The stylesheet is a package this app imports, and until it is imported the
 * tokens resolve to nothing. Leaving the slot out is what keeps a fallback
 * palette from being written down here and then disagreeing with the real one.
 */
test("a token the stylesheet has not defined is left out rather than filled in", () =>
{
    const appearance = terminalAppearance(() => "");

    expect(appearance.theme).toEqual({});
    expect(appearance.fontFamily).toBeUndefined();
    expect(appearance.lineHeight).toBeUndefined();
});
