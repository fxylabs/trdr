import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Nothing Tauri-specific is here yet. The Tauri composition root, the window,
// and the capability file arrive with their own track; this is the plain web
// build they will wrap.
export default defineConfig({
    plugins: [react()],
    build: {
        outDir: "dist",
        emptyOutDir: true
    }
});
