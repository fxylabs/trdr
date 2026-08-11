/**
 * The check that stops the shipped stylesheet from drifting off the design source.
 *
 * `design/ui-kit/` is the authority and `packages/ui/src/styles/` is the copy the
 * application actually loads. Two files in two places is the arrangement that
 * keeps the design source framework-neutral and keeps the app from importing out
 * of a directory that is not a package — and the cost of it is a copy that can
 * quietly stop being one. `src-tauri/tests/bindings.rs` pays the same cost for
 * the generated TypeScript, and this is the same test for CSS: byte for byte,
 * with nothing normalized away and nothing parsed first.
 *
 * The two sides are separate imports of two different paths, so there is no
 * arrangement in which this compares a file to itself.
 */

import { expect, test } from "vitest";

import sourceComponents from "../../../../design/ui-kit/components.v2.css?raw";
import sourceTokens from "../../../../design/ui-kit/tokens.v2.css?raw";
import shippedComponents from "./components.v2.css?raw";
import shippedTokens from "./tokens.v2.css?raw";

/** The files that are copies. `roles.css` is this package's own and is not one. */
const COPIES = [
    ["tokens.v2.css", shippedTokens, sourceTokens],
    ["components.v2.css", shippedComponents, sourceComponents]
] as const;

test.each(COPIES)("%s is byte-for-byte the file in design/ui-kit", (_name, shipped, source) =>
{
    // A pair of empty strings would compare equal and prove nothing.
    expect(source.length).toBeGreaterThan(1000);
    expect(shipped).toBe(source);
});

test("the two copies are two different files", () =>
{
    expect(shippedTokens).not.toBe(shippedComponents);
    expect(shippedTokens).toContain("--trdr-color-focus");
    expect(shippedComponents).toContain(".trdr-app-shell");
});

/**
 * The stylesheet ships classes no component in this package renders.
 *
 * The terminal is the agent track's, and `ApprovalDialog` is M6's. Both style
 * their markup out of this sheet, so a copy that had lost their rules would
 * break a track that never touched this package and has no test here.
 */
test("the classes other tracks style against are present", () =>
{
    for (const owned of [
        ".trdr-terminal-shell",
        ".trdr-terminal-titlebar",
        ".trdr-xterm-host",
        ".trdr-terminal-statusbar",
        ".trdr-agent-rail",
        ".trdr-dialog",
        ".trdr-dialog-actions"
    ])
    {
        expect(shippedComponents).toContain(owned);
    }
});

/** The terminal's seven process states are styled, not just its chrome. */
test("every terminal process state has a rule", () =>
{
    for (const state of [
        "starting",
        "ready",
        "running",
        "needs-input",
        "approval-pending",
        "exited",
        "reconnecting"
    ])
    {
        expect(shippedComponents).toContain(`[data-process="${state}"]`);
    }
});
