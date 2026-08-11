import { expect, test, vi } from "vitest";

import type { TerminalProcess } from "../bindings";
import type { ProcessState } from "./processState";
import {
    FIXTURE_STATES,
    INITIAL_PROCESS_STATE,
    fixtureIsAvailable,
    liveProducer,
    nextFixture
} from "./processState";

function state(process: TerminalProcess, origin: "live" | "synthetic"): ProcessState
{
    return { process, origin };
}

/** The seven the visual contract fixes, and no eighth. */
test("the fixture walks every state the contract names", () =>
{
    expect([...FIXTURE_STATES].sort()).toEqual([
        "approval-pending",
        "exited",
        "needs-input",
        "ready",
        "reconnecting",
        "running",
        "starting"
    ]);

    let current = INITIAL_PROCESS_STATE;
    const seen = new Set<TerminalProcess>();

    for (let step = 0; step < FIXTURE_STATES.length; step += 1)
    {
        const next = nextFixture(current);

        expect(next).not.toBeNull();
        current = next ?? current;
        seen.add(current.process);
    }

    expect(seen.size).toBe(7);
});

/**
 * The two states nothing else can produce. `needs-input` would need the stream
 * to be read, and `approval-pending` needs the approval round-trip in M6, so
 * the fixture is the only thing in M2 that can show either.
 */
test("the two states the host cannot observe come only from the fixture", () =>
{
    const fromFixture = new Set<TerminalProcess>();
    let current = INITIAL_PROCESS_STATE;

    for (let step = 0; step < FIXTURE_STATES.length; step += 1)
    {
        current = nextFixture(current) ?? current;
        fromFixture.add(current.process);
    }

    expect(fromFixture.has("needs-input")).toBe(true);
    expect(fromFixture.has("approval-pending")).toBe(true);
});

/** Everything the fixture produces says so, and cannot be rendered without it. */
test("a synthetic state carries its origin", () =>
{
    expect(nextFixture(INITIAL_PROCESS_STATE)?.origin).toBe("synthetic");
});

/**
 * The rule that keeps a real session out of a made-up state. It is in the model
 * rather than in a handler, so no control anyone wires up later can get around
 * it.
 */
test("the fixture is refused while a session is live", () =>
{
    for (const live of ["starting", "ready", "running", "reconnecting"] as const)
    {
        expect(nextFixture(state(live, "live"))).toBeNull();
        expect(fixtureIsAvailable(state(live, "live"))).toBe(false);
    }

    expect(nextFixture(state("exited", "live"))).not.toBeNull();
    expect(fixtureIsAvailable(state("exited", "live"))).toBe(true);
});

/**
 * A live state always takes effect, including one that lands while the fixture
 * is part-way through its walk. The producer has no opinion about what it is
 * replacing.
 */
test("the live producer reports what the host said, marked as observed", () =>
{
    const publish = vi.fn();
    const listeners: ((process: TerminalProcess) => void)[] = [];
    const stop = liveProducer((handler) =>
    {
        listeners.push(handler);

        return () =>
        {
            listeners.length = 0;
        };
    }).subscribe(publish);

    listeners[0]?.("running");

    expect(publish).toHaveBeenCalledWith({ process: "running", origin: "live" });

    stop();
    expect(listeners).toHaveLength(0);
});
