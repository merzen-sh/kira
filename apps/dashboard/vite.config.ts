import { devtools } from "@tanstack/devtools-vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const src = new URL("./src", import.meta.url).pathname;

export default defineConfig({
  plugins: [devtools(), react()],
  resolve: {
    alias: [{ find: /^@\//, replacement: `${src}/` }],
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:3001",
      "/ws": { target: "ws://127.0.0.1:3001", ws: true },
    },
  },
});
