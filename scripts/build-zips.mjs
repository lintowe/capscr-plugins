#!/usr/bin/env node
// Build each plugin and pack it into a release zip, compute sha256, update
// registry.json. Run from repo root: `node scripts/build-zips.mjs`.
//
// WASM plugins (plugin.toml has `type = "wasm"`): this compiles the crate to
// wasm32-unknown-unknown and includes the resulting module as `plugin.wasm`.
//   Prereq: `rustup target add wasm32-unknown-unknown`.
// Metadata-only plugins (no wasm runtime — e.g. sounds, hotbar pending audio/UI
// host capabilities): packed with just their manifest + docs.
//
// Files in each zip: plugin.toml (required), plugin.wasm (wasm plugins),
// README.md (if present), assets/** (if present).
// Excluded: src/, Cargo.toml, Cargo.lock, target/.
//
// Pure-JS zip writer (zlib.deflateRawSync + manual ZIP container) so there's no
// tar/zip/7z dependency. APPNOTE.TXT §4 subset: STORE/DEFLATE, no zip64.

import { deflateRawSync } from "node:zlib";
import { createHash } from "node:crypto";
import { execSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const distDir = join(root, "dist");
const registryPath = join(root, "registry.json");
const WASM_TARGET = "wasm32-unknown-unknown";

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    t[n] = c >>> 0;
  }
  return t;
})();

const registry = JSON.parse(readFileSync(registryPath, "utf8"));

// a plugin is a WASM plugin if its manifest declares `type = "wasm"`. We read
// the raw text rather than parse TOML to avoid a dependency.
function isWasmPlugin(pluginDir) {
  const manifest = join(pluginDir, "plugin.toml");
  if (!existsSync(manifest)) return false;
  return /type\s*=\s*"wasm"/.test(readFileSync(manifest, "utf8"));
}

// package name convention: crate = `capscr-<id>`; the wasm artifact replaces
// dashes with underscores → `capscr_<id>.wasm`.
const crateName = (id) => `capscr-${id}`;
const wasmArtifact = (id) =>
  join(
    root,
    "target",
    WASM_TARGET,
    "release",
    `capscr_${id.replace(/-/g, "_")}.wasm`,
  );

// compile every wasm plugin once, up front, in a single cargo invocation.
const wasmPlugins = registry.plugins.filter((e) =>
  isWasmPlugin(join(root, e.id)),
);
if (wasmPlugins.length > 0) {
  const pkgArgs = wasmPlugins.map((e) => `-p ${crateName(e.id)}`).join(" ");
  const cmd = `cargo build --release --target ${WASM_TARGET} ${pkgArgs}`;
  console.log(`[build] ${cmd}`);
  execSync(cmd, { cwd: root, stdio: "inherit" });
}

if (existsSync(distDir)) {
  rmSync(distDir, { recursive: true, force: true });
}
mkdirSync(distDir, { recursive: true });

const INCLUDE_FILES = ["plugin.toml", "README.md"];

let registryDirty = false;

for (const entry of registry.plugins) {
  const pluginDir = join(root, entry.id);
  if (!existsSync(pluginDir)) {
    console.warn(`[skip] ${entry.id}: directory missing`);
    continue;
  }
  if (!existsSync(join(pluginDir, "plugin.toml"))) {
    console.warn(`[skip] ${entry.id}: plugin.toml missing`);
    continue;
  }

  const files = [];
  for (const name of INCLUDE_FILES) {
    const p = join(pluginDir, name);
    if (existsSync(p)) {
      files.push({ name, body: readFileSync(p) });
    }
  }

  // wasm plugins: include the compiled module as plugin.wasm
  if (isWasmPlugin(pluginDir)) {
    const wasmPath = wasmArtifact(entry.id);
    if (!existsSync(wasmPath)) {
      console.warn(
        `[skip] ${entry.id}: ${wasmPath} not found — did the build fail?`,
      );
      continue;
    }
    files.push({ name: "plugin.wasm", body: readFileSync(wasmPath) });
  }

  const assetsDir = join(pluginDir, "assets");
  if (existsSync(assetsDir) && statSync(assetsDir).isDirectory()) {
    for (const rel of walk(assetsDir, "")) {
      const abs = join(assetsDir, rel);
      files.push({
        name: `assets/${rel.split(sep).join("/")}`,
        body: readFileSync(abs),
      });
    }
  }

  const zipBytes = buildZip(files);
  const zipPath = join(distDir, `${entry.id}-${entry.version}.zip`);
  writeFileSync(zipPath, zipBytes);

  const sha = createHash("sha256").update(zipBytes).digest("hex");
  const size = zipBytes.length;

  if (entry.sha256 !== sha || entry.size_bytes !== size) {
    entry.sha256 = sha;
    entry.size_bytes = size;
    registryDirty = true;
  }

  console.log(
    `[ok]   ${entry.id}-${entry.version}.zip · ${size} bytes · ${sha.slice(0, 12)}…`,
  );
}

if (registryDirty) {
  registry.updated_unix = Math.floor(Date.now() / 1000);
  writeFileSync(registryPath, JSON.stringify(registry, null, 2) + "\n");
  console.log("[ok]   registry.json updated");
} else {
  console.log("[ok]   registry.json already in sync");
}

// --- pure-JS zip writer ---------------------------------------------------

function buildZip(files) {
  const parts = [];
  const central = [];
  let offset = 0;
  for (const f of files) {
    const nameBytes = Buffer.from(f.name, "utf8");
    const compressed = deflateRawSync(f.body);
    const crc = crc32(f.body);
    const useDeflate = compressed.length < f.body.length;
    const method = useDeflate ? 8 : 0;
    const payload = useDeflate ? compressed : f.body;

    const lfh = Buffer.alloc(30);
    lfh.writeUInt32LE(0x04034b50, 0);
    lfh.writeUInt16LE(20, 4);
    lfh.writeUInt16LE(0, 6);
    lfh.writeUInt16LE(method, 8);
    lfh.writeUInt16LE(0, 10);
    lfh.writeUInt16LE(0, 12);
    lfh.writeUInt32LE(crc, 14);
    lfh.writeUInt32LE(payload.length, 18);
    lfh.writeUInt32LE(f.body.length, 22);
    lfh.writeUInt16LE(nameBytes.length, 26);
    lfh.writeUInt16LE(0, 28);
    parts.push(lfh, nameBytes, payload);

    const cdh = Buffer.alloc(46);
    cdh.writeUInt32LE(0x02014b50, 0);
    cdh.writeUInt16LE(20, 4);
    cdh.writeUInt16LE(20, 6);
    cdh.writeUInt16LE(0, 8);
    cdh.writeUInt16LE(method, 10);
    cdh.writeUInt16LE(0, 12);
    cdh.writeUInt16LE(0, 14);
    cdh.writeUInt32LE(crc, 16);
    cdh.writeUInt32LE(payload.length, 20);
    cdh.writeUInt32LE(f.body.length, 24);
    cdh.writeUInt16LE(nameBytes.length, 28);
    cdh.writeUInt16LE(0, 30);
    cdh.writeUInt16LE(0, 32);
    cdh.writeUInt16LE(0, 34);
    cdh.writeUInt16LE(0, 36);
    cdh.writeUInt32LE(0, 38);
    cdh.writeUInt32LE(offset, 42);
    central.push(cdh, nameBytes);

    offset += lfh.length + nameBytes.length + payload.length;
  }

  const centralOffset = offset;
  const centralBuf = Buffer.concat(central);
  parts.push(centralBuf);

  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(0, 4);
  eocd.writeUInt16LE(0, 6);
  eocd.writeUInt16LE(files.length, 8);
  eocd.writeUInt16LE(files.length, 10);
  eocd.writeUInt32LE(centralBuf.length, 12);
  eocd.writeUInt32LE(centralOffset, 16);
  eocd.writeUInt16LE(0, 20);
  parts.push(eocd);

  return Buffer.concat(parts);
}

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function walk(dir, prefix) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    const rel = prefix ? join(prefix, name) : name;
    const s = statSync(abs);
    if (s.isDirectory()) {
      out.push(...walk(abs, rel));
    } else {
      out.push(rel);
    }
  }
  return out;
}
