/**
 * The coverage page: every role, in every state its contract gives it, plus the
 * five approved views composed out of them.
 *
 * The state grid is generated from the same catalogue the tests assert against,
 * so what a person looks at here and what CI checks cannot come apart — a state
 * that is in `contracts.v2.json` and missing from the catalogue fails
 * `contracts.test.tsx` before it can be missing from this page.
 *
 * This is the artifact the visual gate at 1440×900 is run against. It is not
 * part of the application.
 */

import { useState } from "react";
import type { ReactNode } from "react";

import { AppShell, BrokerConnection, Sidebar } from "../src/index";
import { CATALOGUE } from "../src/testing/catalogue";
import { CONTRACTS } from "../src/testing/contractSource";
import { RECIPES } from "../src/testing/recipes";

type Tab = "states" | "recipes";

const NAVIGATION = [
    { id: "today", label: "Today", href: "#today", current: true },
    { id: "lab", label: "Lab", href: "#lab" },
    { id: "strategies", label: "Strategies", href: "#strategies" }
];

function GallerySidebar()
{
    return (
        <Sidebar
            brand="trdr"
            items={NAVIGATION}
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
    );
}

/** The rail is the agent track's. Here it is a labelled hole of the right size. */
function RailPlaceholder()
{
    return <div className="g-rail">agent rail · 420px{"\n"}terminal is Track B</div>;
}

function Board(props: { children: ReactNode })
{
    return (
        <div className="g-boardbox">
            <div className="g-board">
                <AppShell activeArea="today" sidebar={<GallerySidebar />} agentRail={<RailPlaceholder />}>
                    <div className="g-stack">{props.children}</div>
                </AppShell>
            </div>
        </div>
    );
}

function States()
{
    return (
        <div className="g-page">
            {CATALOGUE.map((role) => (
                <section key={role.contract} className="g-role">
                    <h2>{role.contract}</h2>
                    <p>
                        {`.${CONTRACTS.components[role.contract]?.class ?? ""} · `}
                        {(CONTRACTS.components[role.contract]?.rules ?? []).join(" · ")}
                    </p>

                    <div className="g-cases">
                        {role.cases.map((one) => (
                            <article
                                key={one.state ?? "default"}
                                className={role.wide === true ? "g-case g-case--wide" : "g-case"}
                            >
                                <header>
                                    <b>{one.state ?? "default"}</b>
                                    <span>{one.absent === true ? "renders nothing" : ""}</span>
                                </header>

                                {role.contract === "AppShell" ? (
                                    <div className="g-frame">
                                        <div className="trdr-scope">{one.element}</div>
                                    </div>
                                ) : (
                                    <div>
                                        <div className="trdr-scope">{one.element}</div>
                                    </div>
                                )}
                            </article>
                        ))}
                    </div>
                </section>
            ))}
        </div>
    );
}

function Recipes()
{
    return (
        <div className="g-page">
            {Object.entries(RECIPES).map(([name, View]) => (
                <section key={name} className="g-role">
                    <h2>{name}</h2>
                    <p>{(CONTRACTS.recipes[name] ?? []).join(" · ")}</p>

                    <Board>
                        <View />
                    </Board>
                </section>
            ))}
        </div>
    );
}

export function Gallery()
{
    // The hash so the page can be opened straight onto either half — a
    // screenshot run has no one to click the tab for it.
    const [tab, setTab] = useState<Tab>(() => (window.location.hash === "#recipes" ? "recipes" : "states"));

    function show(next: Tab)
    {
        window.location.hash = next;
        setTab(next);
    }

    return (
        <>
            <header className="g-bar">
                <span className="g-mark">@trdr/ui · coverage</span>

                <nav>
                    <button type="button" aria-pressed={tab === "states"} onClick={() => show("states")}>
                        1 · Roles and states
                    </button>
                    <button type="button" aria-pressed={tab === "recipes"} onClick={() => show("recipes")}>
                        2 · Composed views
                    </button>
                </nav>

                <p className="g-note">
                    tokens.v2.css → components.v2.css → roles.css
                    <br />
                    states generated from contracts.v2.json · page chrome is `g-` and ships nothing
                </p>
            </header>

            {tab === "states" ? <States /> : <Recipes />}
        </>
    );
}
