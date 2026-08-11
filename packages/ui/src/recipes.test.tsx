/**
 * The stop condition: every approved view composes out of what is shipped, and
 * none of them needs a rule of its own.
 *
 * The second half is the one worth testing mechanically. Every class name in a
 * rendered view is collected and checked against the class names the shipped
 * stylesheet actually defines — so a screen that reached for `today-card`,
 * `lab-grid` or any other name invented on the spot fails here, which is exactly
 * the failure this package exists to make impossible.
 */

import { cleanup, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, expect, test } from "vitest";

import { AppShell, BrokerConnection, Sidebar } from "./index";
import { CONTRACTS } from "./testing/contractSource";
import { RECIPES } from "./testing/recipes";
import { KIT_CSS, rules } from "./testing/stylesheet";

afterEach(cleanup);

/** Every class name the stylesheet defines a rule for. */
const DEFINED = new Set(
    rules(KIT_CSS).flatMap((rule) => [...rule.selector.matchAll(/\.([\w-]+)/g)].map(([, name]) => name!))
);

/** The two recipes that belong to other tracks. */
const ELSEWHERE = ["PersistentAgentRail", "DurableMutation"];

function shell(children: ReactNode)
{
    return (
        <AppShell
            activeArea="today"
            sidebar={
                <Sidebar
                    brand="trdr"
                    items={[
                        { id: "today", label: "Today", href: "#/", current: true },
                        { id: "lab", label: "Lab", href: "#/lab" },
                        { id: "strategies", label: "Strategies", href: "#/strategies" }
                    ]}
                    connection={
                        <BrokerConnection
                            broker="KIS"
                            state="connected-read-only"
                            stateLabel="연결됨"
                            authority="읽기 전용"
                            lastSync="14:31 동기화"
                        />
                    }
                />
            }
            agentRail={<div />}
        >
            {children}
        </AppShell>
    );
}

test("the recipes this package is responsible for are the ones it composes", () =>
{
    expect([...Object.keys(RECIPES), ...ELSEWHERE].sort()).toEqual(Object.keys(CONTRACTS.recipes).sort());
});

test.each(Object.entries(RECIPES))("%s renders inside the shell", (name, View) =>
{
    render(shell(<View />));

    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(screen.getByRole("main").textContent).not.toBe("");
    expect(name).toBeDefined();
});

test.each(Object.entries(RECIPES))("%s invents no class of its own", (_name, View) =>
{
    const { container } = render(shell(<View />));
    const used = new Set(
        [...container.querySelectorAll("[class]")].flatMap((element) => [...element.classList])
    );

    expect([...used].filter((name) => !DEFINED.has(name)).sort()).toEqual([]);
});

/**
 * The roles each recipe names are the roles that exist.
 *
 * The contract writes them with decorations — `DataTable+StockCell`, `RuleGrid
 * frozen`, `Panel chart`, `TopBar+BackButton` — so a `+` is two roles and a
 * trailing word is a qualifier on one. `BackButton` is `TopBar`'s `back` prop
 * and is named here rather than skipped.
 */
test("every role a recipe names is a role the contract has", () =>
{
    const ALIASES: Readonly<Record<string, string>> = { BackButton: "TopBar" };
    const named = Object.entries(CONTRACTS.recipes)
        .filter(([recipe]) => !ELSEWHERE.includes(recipe))
        .flatMap(([, roles]) => roles)
        .flatMap((role) => role.split("+"))
        .map((role) => role.trim().split(" ")[0] ?? "")
        .map((role) => ALIASES[role] ?? role);

    for (const role of named)
    {
        expect([role, role in CONTRACTS.components]).toEqual([role, true]);
    }
});
