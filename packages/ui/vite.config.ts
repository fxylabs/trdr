/// <reference types="vitest/config" />
import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

/**
 * The package builds nothing the application consumes.
 *
 * `@trdr/ui` is source: the desktop app imports `src/index.ts` through the
 * workspace link and its own Vite build compiles it. What this config exists for
 * is the two things that are local to the package — the coverage gallery, which
 * is a development page and never enters the shipped bundle, and vitest.
 */
export default defineConfig({
    plugins: [react()],

    root: fileURLToPath(new URL("./gallery", import.meta.url)),

    build: {
        // Same floor as the desktop app: the packaged WebView is Safari 16.
        target: "safari15",
        // `dist/` rather than a name of its own, because that is what the
        // repository's .gitignore already knows not to commit.
        outDir: fileURLToPath(new URL("./dist", import.meta.url)),
        emptyOutDir: true
    },

    test: {
        root: fileURLToPath(new URL(".", import.meta.url)),
        environment: "jsdom",
        include: ["src/**/*.test.{ts,tsx}"],
        css: true
    }
});
