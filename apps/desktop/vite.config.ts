/// <reference types="vitest/config" />
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
    plugins: [react()],

    // Tauri starts this server and then points the WebView at a fixed address,
    // so moving to another port when 1420 is taken would leave the window
    // looking at nothing. `tauri.conf.json`'s `devUrl` is the other half of this
    // pair and has to say the same number.
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true
    },

    build: {
        // macOS 13 is the floor `tauri.conf.json` sets, and its WebView is
        // Safari 16. Targeting 15 leaves a version of room and keeps esbuild
        // from emitting syntax the packaged WebView would only fail on at
        // runtime — where a browser-based `vite preview` would never show it.
        target: "safari15",
        // Read by `frontendDist` in `tauri.conf.json`.
        outDir: "dist",
        emptyOutDir: true
    },

    test: {
        environment: "jsdom",
        include: ["src/**/*.test.{ts,tsx}"],
        css: true
    }
});
