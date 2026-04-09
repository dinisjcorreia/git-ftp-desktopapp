#!/usr/bin/env node

import { spawn } from "node:child_process";
import { constants, accessSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(scriptDir, "..");
const tauriCliEntry = path.join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const tauriArgs = process.argv.slice(2);

try {
  accessSync(tauriCliEntry, constants.R_OK);
} catch {
  console.error(
    "Could not find the Tauri CLI entrypoint at 'node_modules/@tauri-apps/cli/tauri.js'. Run 'npm install' first.",
  );
  process.exit(1);
}

const child = spawn(process.execPath, [tauriCliEntry, ...tauriArgs], {
  cwd: rootDir,
  env: process.env,
  stdio: "inherit",
});

child.on("error", (error) => {
  console.error(`Failed to launch the Tauri CLI: ${error.message}`);
  process.exit(1);
});

child.on("close", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
