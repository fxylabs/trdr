import type { RouteObject } from "react-router";
import type { WorkArea } from "@trdr/ui";

import { NAV } from "../copy/ko";
import { LabDraft } from "../screens/LabDraft";
import { LabResult } from "../screens/LabResult";
import { Strategies } from "../screens/Strategies";
import { StrategyDetail } from "../screens/StrategyDetail";
import { Today } from "../screens/Today";
import { AppShell } from "./AppShell";

/** Where each screen lives, in one place, so a link and a route cannot disagree. */
export const paths = {
    today: "/",
    lab: "/lab",
    labResult: "/lab/result",
    strategies: "/strategies",
    strategy: "/strategies/:strategy"
} as const;

/**
 * One strategy's own address.
 *
 * The detail view has a route rather than a piece of list state because its
 * recipe includes a back control, and a view that was pushed into place has
 * nowhere to go back to. It is also what section 9.2's `trdr ui open` will point
 * at, so the id belongs in the URL rather than in a variable.
 */
export function strategyPath(strategy: string): string
{
    return `/strategies/${encodeURIComponent(strategy)}`;
}

/** What the navigation shows, in the order it shows it. */
export const navigation: readonly { area: WorkArea; path: string; label: string }[] = [
    { area: "today", path: paths.today, label: NAV.today },
    { area: "lab", path: paths.lab, label: NAV.lab },
    { area: "strategies", path: paths.strategies, label: NAV.strategies }
];

/**
 * Which of the three areas a location is in.
 *
 * Prefix rather than equality, because two of the areas have a second screen
 * under them — the Lab's result and one strategy's detail — and the navigation
 * has three entries by contract. A detail view that left the sidebar showing no
 * current item would be telling the user they had left the application.
 */
export function areaOf(pathname: string): WorkArea
{
    if (pathname.startsWith(paths.strategies))
    {
        return "strategies";
    }

    return pathname.startsWith(paths.lab) ? "lab" : "today";
}

/**
 * The route table.
 *
 * The shape is the load-bearing part, not the contents. `AppShell` is a layout
 * route: it has no path of its own, and every screen is a child of it. React
 * Router swaps what `<Outlet/>` renders and leaves the element around it alone,
 * so the terminal host inside `AppShell` is mounted once for the life of the
 * window rather than once per navigation. Section 14 item 3 calls that host
 * persistent, and this nesting is what makes it so.
 *
 * The Lab is two routes and one area. Section 11 gives the draft and the result
 * two models and `contracts.v2.json` gives them two recipes, so they are two
 * views; putting them behind one path with a switch inside would make the result
 * unreachable by `trdr ui open` and unaddressable by a reload.
 *
 * Exported rather than inlined into the router so the tests can mount the same
 * table through a memory router. A test that declared its own routes would be
 * proving something about its own tree.
 */
export const routes: RouteObject[] = [
    {
        element: <AppShell />,
        children: [
            { path: paths.today, element: <Today /> },
            { path: paths.lab, element: <LabDraft /> },
            { path: paths.labResult, element: <LabResult /> },
            { path: paths.strategies, element: <Strategies /> },
            { path: paths.strategy, element: <StrategyDetail /> }
        ]
    }
];
