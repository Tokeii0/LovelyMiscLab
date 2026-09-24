import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

// Unit tests for pure frontend logic (stores, helpers). Node environment on
// purpose: no DOM, so anything touching `window`/`document` stays out of tests.
export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  define: {
    __APP_VERSION__: JSON.stringify("test"),
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
