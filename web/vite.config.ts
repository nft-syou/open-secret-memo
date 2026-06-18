import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  base: "/",
  plugins: [
    react(),
    VitePWA({
      registerType: "autoUpdate",
      includeAssets: ["favicon.svg"],
      workbox: {
        // Precache the wasm so decrypt/encrypt work fully offline.
        globPatterns: ["**/*.{js,css,html,wasm,svg}"],
        maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
      },
      manifest: {
        name: "Open Secret Memo",
        short_name: "OSM",
        description: "ブラウザ内だけで秘密メモを暗号化・復号",
        theme_color: "#0f172a",
        background_color: "#0f172a",
        display: "standalone",
        start_url: "/",
        icons: [
          { src: "icon-192.png", sizes: "192x192", type: "image/png" },
          { src: "icon-512.png", sizes: "512x512", type: "image/png" }
        ]
      }
    })
  ],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    // Unit tests live under src/. Playwright e2e specs (e2e/*.spec.ts) import
    // @playwright/test and must NOT be collected by Vitest — Playwright runs
    // them via its own config (testDir "./e2e", `pnpm test:e2e`).
    include: ["src/**/*.{test,spec}.{ts,tsx}"]
  }
});
