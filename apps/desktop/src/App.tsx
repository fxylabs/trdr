import { RouterProvider, createHashRouter } from "react-router";

import { routes } from "./shell/routes";

/**
 * Why React Router, and why the hash.
 *
 * React Router is here for one feature: layout routes. A parent route with an
 * `<Outlet/>` stays mounted while its children swap, which is exactly the
 * lifetime the terminal host needs and the thing a router without nesting cannot
 * give without a portal.
 *
 * The hash, because in a packaged build the WebView loads from `tauri://` and
 * asks that protocol for whatever path is in the URL. There is no server behind
 * it to fall back to `index.html`, so a reload on `/lab` would ask for a file
 * that was never built. With `#/lab` the path the protocol sees is always
 * `index.html`, and the route lives in the fragment, which is never requested.
 * The URL stays a real one, which matters later: the `trdr` CLI's `ui.open`
 * (section 9.2) needs somewhere to point.
 */
const router = createHashRouter(routes);

export function App()
{
    return <RouterProvider router={router} />;
}
