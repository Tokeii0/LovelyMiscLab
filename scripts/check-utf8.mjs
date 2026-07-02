import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { TextDecoder } from "node:util";

const root = process.cwd();
const skip = new Set([".git", "node_modules", "target", "dist"]);
const exts = new Set([
  ".css",
  ".html",
  ".json",
  ".md",
  ".mjs",
  ".rs",
  ".toml",
  ".ts",
  ".tsx",
]);

const decoder = new TextDecoder("utf-8", { fatal: true });
const bad = [];

function extOf(path) {
  const i = path.lastIndexOf(".");
  return i >= 0 ? path.slice(i) : "";
}

function walk(dir) {
  for (const name of readdirSync(dir)) {
    if (skip.has(name)) continue;
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      walk(path);
      continue;
    }
    if (!exts.has(extOf(path))) continue;
    const rel = relative(root, path);
    try {
      const text = decoder.decode(readFileSync(path));
      if (text.includes("\uFFFD")) bad.push(`${rel}: contains replacement character`);
    } catch (error) {
      bad.push(`${rel}: ${error.message}`);
    }
  }
}

walk(root);

if (bad.length > 0) {
  console.error("UTF-8 check failed:");
  for (const item of bad) console.error(`- ${item}`);
  process.exit(1);
}

console.log("UTF-8 check passed.");
