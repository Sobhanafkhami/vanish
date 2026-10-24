// Stamps one version into package.json, tauri.conf.json and Cargo.toml (used by CI; not committed back).
// Usage: node scripts/set-version.mjs 0.1.42
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("usage: node scripts/set-version.mjs <major.minor.patch>");
  process.exit(1);
}

function rewriteJson(path) {
  const raw = readFileSync(path, "utf8");
  const next = raw.replace(/("version"\s*:\s*)"[^"]*"/, `$1"${version}"`);
  if (next === raw && !raw.includes(`"version": "${version}"`)) throw new Error(`no version field in ${path}`);
  writeFileSync(path, next);
}
rewriteJson("package.json");
rewriteJson("src-tauri/tauri.conf.json");

// Only the [package] version, not dependency versions.
const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const nextCargo = cargo.replace(/(\[package\][^[]*?\nversion\s*=\s*)"[^"]*"/, `$1"${version}"`);
if (nextCargo === cargo && !cargo.includes(`version = "${version}"`)) throw new Error("no [package] version in Cargo.toml");
writeFileSync(cargoPath, nextCargo);

console.log(`version set to ${version}`);
