#!/usr/bin/env node
// Watch the shared source + the Firefox manifest and rebuild the .xpi on every
// change. No deps — just node's fs.watch with a small debounce so a single save
// triggers exactly one rebuild.
//
//   node scripts/watch-xpi.js   ->   rebuilds extension/dist/tailscale-screenshot.xpi
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const ext = path.resolve(__dirname, "..");
const watched = [path.join(ext, "src"), path.join(ext, "manifest.firefox.json")];

function rebuild() {
  const t = new Date().toISOString();
  process.stdout.write(`[${t}] change detected, rebuilding .xpi…\n`);
  const r = spawnSync(path.join(ext, "scripts", "package.sh"), ["firefox"], {
    stdio: "inherit",
  });
  if (r.status !== 0) process.stderr.write("rebuild failed\n");
}

let timer = null;
function schedule() {
  clearTimeout(timer);
  timer = setTimeout(rebuild, 150);
}

for (const target of watched) {
  if (!fs.existsSync(target)) continue;
  const opts = fs.statSync(target).isDirectory() ? { recursive: true } : {};
  fs.watch(target, opts, schedule);
}

process.stdout.write("watching for changes (Ctrl+C to stop)…\n");
rebuild();
