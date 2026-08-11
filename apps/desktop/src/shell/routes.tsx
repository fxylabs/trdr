import type { RouteObject } from "react-router";

import { Lab } from "../screens/Lab";
import { Strategies } from "../screens/Strategies";
import { Today } from "../screens/Today";
import { AppShell } from "./AppShell";

/** Where each screen lives, in one place, so a link and a route cannot disagree. */
export const paths = {
    today: "/",
    lab: "/lab",
    strategies: "/strategies"
} as const;

/** What the navigation shows, in the order it shows it. */
export const navigation = [
    { path: paths.today, label: "Today" },
    { path: paths.lab, label: "Lab" },
    { path: paths.strategies, label: "Strategies" }
] as const;

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
 * Exported rather than inlined into the router so the tests can mount the same
 * table through a memory router. A test that declared its own routes would be
 * proving something about its own tree.
 */
export const routes: RouteObject[] = [
    {
        element: <AppShell />,
        children: [
            { path: paths.today, element: <Today /> },
            { path: paths.lab, element: <Lab /> },
            { path: paths.strategies, element: <Strategies /> }
        ]
    }
];
