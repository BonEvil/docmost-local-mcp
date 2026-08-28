#!/usr/bin/env node

console.error(
  "docmost-local-mcp: automatic binary download is disabled in the hardened fork.\n" +
  "Use scripts/install-atlas.sh with a reviewed commit, digest, and signed release manifest.",
);
process.exit(1);
