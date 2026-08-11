/**
 * The terminal's colours, taken from the token block and from nowhere else.
 *
 * `design/ui-kit/tokens.v2.css` owns every colour in the rail. Not one of them
 * is written down in this file: what is written down is the *name* of each
 * token and which slot of xterm's theme it fills, and the values are read off
 * the live element. A colour that changed in the token block changes here with
 * no edit, and a colour that was never in the token block cannot appear.
 *
 * # What may be themed, and what may not
 *
 * `XtermHost`'s contract draws the line: base ANSI colours may be themed, and
 * explicit true-colour sequences are preserved. That is exactly what this map
 * is — the sixteen base slots, the background, the foreground, the cursor, and
 * the selection. An agent that emits `ESC [ 38;2;12;34;56 m` names a colour
 * itself, xterm uses the value it named, and nothing here is consulted.
 *
 * # Why a reader is passed in
 *
 * The tokens live in CSS custom properties, and reading one means asking the
 * browser for a computed style. Taking the reader as an argument keeps that
 * one call in [`tokensOf`] and lets a test hand over a map instead of arranging
 * a stylesheet — a test that had to load the real one would be checking that
 * the browser resolves `var()`.
 */

/** Reads one CSS custom property, and answers with the empty string if it has none. */
export type TokenReader = (name: string) => string;

/**
 * Which token fills which slot of xterm's theme.
 *
 * The eight base ANSI colours are the ones the gallery's palette panel shows.
 * The bright half is deliberately not eight more tokens: the palette has eight
 * hues, and inventing eight more here would be writing colours rather than
 * reading them. `brightBlack` and `brightWhite` are the two the token block
 * already has an answer for — the dim and strong foregrounds.
 */
const SLOTS: Readonly<Record<string, string>> = {
    background: "--trdr-terminal-background",
    foreground: "--trdr-terminal-foreground",
    cursor: "--trdr-terminal-cursor",
    cursorAccent: "--trdr-terminal-background",
    selectionBackground: "--trdr-terminal-selection",
    black: "--trdr-terminal-background",
    red: "--trdr-terminal-error",
    green: "--trdr-terminal-success",
    yellow: "--trdr-terminal-warning",
    blue: "--trdr-terminal-info",
    magenta: "--trdr-terminal-magenta",
    cyan: "--trdr-terminal-cyan",
    white: "--trdr-terminal-foreground",
    brightBlack: "--trdr-terminal-dim",
    brightWhite: "--trdr-terminal-strong"
};

/** The tokens that decide the terminal's type, in the same arrangement. */
const TYPE_SLOTS = {
    fontFamily: "--trdr-font-mono",
    lineHeight: "--trdr-leading-terminal"
} as const;

/** What xterm is given. Only the slots the token block actually answered for. */
export interface TerminalAppearance
{
    /** The colour slots that resolved. */
    readonly theme: Readonly<Record<string, string>>;
    /** The monospace stack, when the token block names one. */
    readonly fontFamily?: string;
    /** The line height, when the token block names one. */
    readonly lineHeight?: number;
}

/** Reads the custom properties that apply to this element. */
export function tokensOf(element: Element): TokenReader
{
    const style = getComputedStyle(element);

    return (name) => style.getPropertyValue(name).trim();
}

/**
 * The appearance the token block describes.
 *
 * A token that resolves to nothing is left out rather than filled in. Until the
 * kit's stylesheet is loaded that means xterm keeps its own defaults, which is
 * the honest outcome — the alternative is a copy of the palette in this file,
 * quietly disagreeing with the real one from the day it was written.
 */
export function terminalAppearance(read: TokenReader): TerminalAppearance
{
    const theme: Record<string, string> = {};

    for (const [slot, token] of Object.entries(SLOTS))
    {
        const value = read(token);

        if (value !== "")
        {
            theme[slot] = value;
        }
    }

    const fontFamily = read(TYPE_SLOTS.fontFamily);
    const lineHeight = Number.parseFloat(read(TYPE_SLOTS.lineHeight));

    return {
        theme,
        ...(fontFamily === "" ? {} : { fontFamily }),
        ...(Number.isFinite(lineHeight) ? { lineHeight } : {})
    };
}
