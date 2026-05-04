import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import { execSync } from "node:child_process";
import pkg from "./package.json" with { type: "json" };

// Tauri expects the Vite dev server on a fixed port so the Rust
// backend can point its WebView at it. See
// https://v2.tauri.app/start/frontend/sveltekit/
const host = process.env.TAURI_DEV_HOST;

function sourceSha(): string {
  try {
    const sha = execSync("git rev-parse --short HEAD", { encoding: "utf8" }).trim();
    execSync("git diff --quiet", { stdio: "ignore" });
    return sha;
  } catch {
    try {
      const sha = execSync("git rev-parse --short HEAD", { encoding: "utf8" }).trim();
      return `${sha}-dirty`;
    } catch {
      return "unknown";
    }
  }
}

const buildTime = new Date().toISOString();
const buildSha = sourceSha();

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
    __APP_BUILD_ID__: JSON.stringify(`${pkg.version}+${buildSha}`),
    __APP_BUILD_TIME__: JSON.stringify(buildTime),
  },
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Don't watch src-tauri — cargo watch handles that.
      ignored: ["**/src-tauri/**"],
    },
  },
});
