import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@xterm/xterm/css/xterm.css";
import "./styles.css";
import { App } from "./App";

const host = document.getElementById("root");
if (host !== null)
{
    createRoot(host).render(<StrictMode><App /></StrictMode>);
}
