#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const ext = process.platform === "win32" ? ".exe" : "";
const binaryPath = join(__dirname, "bin", `docmost-local-mcp${ext}`);

if (!existsSync(binaryPath)) {
  console.error(
    "docmost-local-mcp: bundled binary not found. The npm downloader is disabled.\n" +
    "Atlas operators must use the verified fork installer documented in " +
    "docs/atlas-release-integrity.md.",
  );
  process.exit(1);
}

try {
  execFileSync(binaryPath, process.argv.slice(2), {
    stdio: "inherit",
    env: process.env,
  });
} catch (error) {
  if (typeof error.status === "number") {
    process.exit(error.status);
  }
  console.error(`Failed to launch ${binaryPath}: ${error.message}`);
  process.exit(1);
}
