import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";

// The UI system first, the shell's four remaining rules after it: `shell.css`
// corrects the frame `.trdr-app-shell` is sized for, and a correction has to
// come after the thing it corrects.
import "@trdr/ui/styles.css";
import "./shell/shell.css";

const root = document.getElementById("root");

if (root === null)
{
    throw new Error("index.html is missing its root element");
}

createRoot(root).render(
    <StrictMode>
        <App />
    </StrictMode>
);
