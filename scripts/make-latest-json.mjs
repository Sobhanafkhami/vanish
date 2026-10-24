// Builds latest.json for the in-app updater from the signed NSIS installer produced by `tauri build`.
// Upload the installer, its .sig and latest.json to a GitHub release tagged v<version>.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const version = conf.version;
const repo = "Sobhanafkhami/vanish";
const dir = join("src-tauri", "target", "release", "bundle", "nsis");
const installer = `Vanish_${version}_x64-setup.exe`;

if (!existsSync(join(dir, `${installer}.sig`))) {
  console.error(
    `Missing ${installer}.sig. Build with the signing key set, e.g.\n` +
      `  $env:TAURI_SIGNING_PRIVATE_KEY = "C:\\Users\\<you>\\.tauri\\vanish-updater.key"; npm run build:app`,
  );
  process.exit(1);
}

const latest = {
  version,
  notes: process.argv[2] ?? `Vanish ${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: readFileSync(join(dir, `${installer}.sig`), "utf8").trim(),
      url: `https://github.com/${repo}/releases/download/v${version}/${installer}`,
    },
  },
};
const out = join(dir, "latest.json");
writeFileSync(out, JSON.stringify(latest, null, 2));
console.log(`wrote ${out}\nUpload ${installer}, ${installer}.sig and latest.json to the v${version} release.`);
