/**
 * The gallery's entry point, and the only place in this package that mounts React.
 *
 * Development only. Nothing here is exported from `src/index.ts`, the desktop app
 * imports none of it, and `vite build` in this package writes to `dist-gallery/`
 * — the shipped bundle is `apps/desktop`'s and never sees this file.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "../src/styles/index.css";
import "./gallery.css";

import { Gallery } from "./Gallery";

const root = document.getElementById("root");

if (root === null)
{
    throw new Error("the gallery page has no #root to mount into");
}

createRoot(root).render(
    <StrictMode>
        <Gallery />
    </StrictMode>
);
