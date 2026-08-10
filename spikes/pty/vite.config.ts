import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri drives the dev server, so the port is fixed and a failure to bind must
// be an error rather than a silent move to another port the host is not watching.
export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server:
    {
        port: 1420,
        strictPort: true
    },
    build:
    {
        target: "safari15"
    }
});
