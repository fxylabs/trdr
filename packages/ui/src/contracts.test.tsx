/**
 * The contract, read from the file that is the contract.
 *
 * Nothing here is written down twice. The class names, the state strings and the
 * component list all come out of `design/ui-kit/contracts.v2.json` at run time
 * and are compared against what the components actually render, so a contract
 * that changes fails this suite rather than silently disagreeing with the code
 * that was written from it.
 */

import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, test } from "vitest";

import {
    APPROVAL_STATES,
    ASYNC_DATA_STATES,
    BROKER_CONNECTION_STATES,
    CONTROL_STATES,
    MARKET_VALUE_TONES,
    PAPER_VALIDATION_STATES,
    TERMINAL_PROCESS_STATES
} from "./index";
import { CATALOGUE } from "./testing/catalogue";
import { CONTRACTS } from "./testing/contractSource";

/** What this package exports for each of the contract's state models. */
const EXPORTED: Readonly<Record<string, readonly string[]>> = {
    control: CONTROL_STATES,
    asyncData: ASYNC_DATA_STATES,
    brokerConnection: BROKER_CONNECTION_STATES,
    terminalProcess: TERMINAL_PROCESS_STATES,
    approval: APPROVAL_STATES,
    paperValidation: PAPER_VALIDATION_STATES,
    marketValue: MARKET_VALUE_TONES
};

afterEach(cleanup);

test("the file this suite reads is the contract and not an empty object", () =>
{
    expect(Object.keys(CONTRACTS.components)).toHaveLength(26);
    expect(Object.keys(CONTRACTS.stateModels)).toHaveLength(7);
});

test("every state model is exported, in full, spelled the contract's way", () =>
{
    expect(Object.keys(EXPORTED).sort()).toEqual(Object.keys(CONTRACTS.stateModels).sort());

    for (const [model, states] of Object.entries(CONTRACTS.stateModels))
    {
        expect(EXPORTED[model]).toEqual(states);
    }
});

/**
 * The roles this package owns.
 *
 * The terminal is the agent track's and `ApprovalDialog` is M6's; both are
 * listed here so that a role added to the contract and to neither list fails
 * this test instead of quietly never being built.
 */
const NOT_OURS = ["TerminalShell", "TerminalTitleBar", "XtermHost", "TerminalStatusBar", "ApprovalDialog"];

test("every contract component is either shipped or explicitly someone else's", () =>
{
    const shipped = CATALOGUE.map((role) => role.contract);

    expect([...shipped, ...NOT_OURS].sort()).toEqual(Object.keys(CONTRACTS.components).sort());
});

test.each(CATALOGUE.map((role) => [role.contract, role] as const))(
    "%s renders the class the contract names",
    (contract, role) =>
    {
        const expected = CONTRACTS.components[contract]?.class;
        const [first] = role.cases;

        expect(expected).toBeDefined();
        expect(first).toBeDefined();

        const { container } = render(first!.element);

        expect(container.querySelector(`.${expected!}`)).not.toBeNull();
    }
);

test.each(CATALOGUE.map((role) => [role.contract, role] as const))(
    "%s covers exactly the states the contract gives it",
    (contract, role) =>
    {
        const covered = [...role.cases.map((one) => one.state), ...(role.pseudoStates ?? [])];

        expect(covered.sort()).toEqual([...(CONTRACTS.components[contract]?.states ?? [])].sort());
    }
);

test.each(
    CATALOGUE.flatMap((role) =>
        role.cases.map((one) => [`${role.contract} · ${one.state ?? "default"}`, role.contract, one] as const)
    )
)("%s writes its state where the stylesheet and a reader can see it", (_name, contract, one) =>
{
    const selector = `.${CONTRACTS.components[contract]!.class}`;
    const { container } = render(one.element);
    const element = container.querySelector(selector);

    if (one.absent === true)
    {
        expect(element).toBeNull();
        return;
    }

    expect(element).not.toBeNull();
    expect(element!.getAttribute("data-state")).toBe(one.state);
});
