import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, test, vi } from "vitest";

import type { TerminalProcess } from "../bindings";
import { FIXTURE_STATES } from "./processState";
import { TerminalStatusBar } from "./TerminalStatusBar";

afterEach(cleanup);

function show(process: TerminalProcess, origin: "live" | "synthetic" = "live")
{
    const onRestart = vi.fn();
    const onStepFixture = vi.fn();

    render(
        <TerminalStatusBar
            state={{ process, origin }}
            context="Today"
            failure={null}
            onRestart={onRestart}
            onStepFixture={onStepFixture}
        />
    );

    return { onRestart, onStepFixture };
}

/**
 * All seven, rendered. The visual contract lists seven states for this
 * component and M2's evidence gate asks for every one of them to be
 * reproducible, so every one of them is asked for here by name.
 */
test("every process state the contract names renders", () =>
{
    for (const process of FIXTURE_STATES)
    {
        show(process);

        expect(screen.getByText(process.toUpperCase(), { exact: false })).toBeDefined();

        cleanup();
    }

    expect(FIXTURE_STATES).toHaveLength(7);
});

/**
 * The contract asks for `aria-live="polite"` on process-state changes and for
 * terminal bytes never to be announced. Both come from where the live region
 * is: it holds the state, and the terminal's subtree is nowhere inside it.
 */
test("the state is announced politely and nothing else is announced at all", () =>
{
    const { container } = render(
        <TerminalStatusBar
            state={{ process: "running", origin: "live" }}
            context="Today"
            failure={null}
            onRestart={() => undefined}
            onStepFixture={() => undefined}
        />
    );

    const announced = container.querySelectorAll("[aria-live]");

    expect(announced).toHaveLength(1);
    expect(announced[0]?.getAttribute("aria-live")).toBe("polite");
    expect(announced[0]?.textContent).toContain("RUNNING");
});

/** A synthetic state says so, wherever it is shown. */
test("a synthetic state is marked as synthetic", () =>
{
    show("needs-input", "synthetic");

    expect(screen.getByText(/NEEDS-INPUT · SYNTHETIC/u)).toBeDefined();
});

/**
 * The refusal that keeps a live session out of a made-up state, as the person
 * sees it: the control is disabled while anything is running, so the reason it
 * cannot be used is on the screen rather than only in the handler.
 */
test("the synthetic control is disabled while a session is live", () =>
{
    show("running");

    expect(screen.getByRole("button", { name: /synthetic/u }).hasAttribute("disabled")).toBe(
        true
    );

    cleanup();
    show("exited");

    expect(screen.getByRole("button", { name: /synthetic/u }).hasAttribute("disabled")).toBe(
        false
    );
});

/** `restart() when exited`, and only then. */
test("restart is offered when the agent has exited", async () =>
{
    const user = userEvent.setup();
    const { onRestart } = show("exited");

    await user.click(screen.getByRole("button", { name: "restart" }));

    expect(onRestart).toHaveBeenCalledTimes(1);

    cleanup();
    show("running");

    expect(screen.queryByRole("button", { name: "restart" })).toBeNull();
});

/**
 * A refusal is rendered as the code section 12 gives it, the way the shell's
 * status line renders `APP_NOT_RUNNING`. There is no sentence here for anything
 * to be written into.
 */
test("a refused start is named by its code", () =>
{
    render(
        <TerminalStatusBar
            state={{ process: "exited", origin: "live" }}
            context="Today"
            failure="TERMINAL_EXECUTABLE_MISSING"
            onRestart={() => undefined}
            onStepFixture={() => undefined}
        />
    );

    expect(screen.getByText(/TERMINAL_EXECUTABLE_MISSING/u)).toBeDefined();
});
