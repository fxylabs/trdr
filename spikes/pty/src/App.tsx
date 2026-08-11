import { useEffect, useState } from "react";

import { Controls } from "./Controls";
import type { Size } from "./pty";
import { TerminalRail } from "./TerminalRail";

const ROUTES = ["today", "lab", "strategies", "rail-hidden"] as const;

type Route = typeof ROUTES[number];

// A hash router rather than a library, because the only thing under test is
// whether a route change unmounts the rail. Anything more would put a library's
// behaviour between the question and the answer.
export function App()
{
    const route = useRoute();
    const [size, setSize] = useState<Size>({ cols: 80, rows: 24 });
    return <div className="shell">
        <nav className="nav">
            {ROUTES.map((name) => <a
                key={name}
                href={`#/${name}`}
                className={name === route ? "nav-item active" : "nav-item"}
            >{name}</a>)}
        </nav>
        <main className="main">
            <h1>{route}</h1>
            {route === "today" ? <Controls size={size} /> : <Filler route={route} />}
        </main>
        <TerminalRail hidden={route === "rail-hidden"} onSize={setSize} />
    </div>;
}

function Filler({ route }: { route: Route })
{
    return <p className="filler">
        Route <code>{route}</code> is rendered by a component that mounts and unmounts.
        The rail beside it does not. Type in the terminal, move here, and move back.
    </p>;
}

function useRoute(): Route
{
    const [route, setRoute] = useState<Route>(read());
    useEffect(() =>
    {
        const update = () => setRoute(read());
        window.addEventListener("hashchange", update);
        return () => window.removeEventListener("hashchange", update);
    }, []);
    return route;
}

function read(): Route
{
    const name = window.location.hash.replace(/^#\//, "");
    return ROUTES.includes(name as Route) ? name as Route : "today";
}
