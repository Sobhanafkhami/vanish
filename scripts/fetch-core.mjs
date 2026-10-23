// Downloads the official sing-box release into src-tauri/binaries so it gets bundled with the app.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, copyFileSync, readdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const VERSION = "1.14.3";
const win = process.platform === "win32";
const ext = win ? "zip" : "tar.gz";
const os = win ? "windows" : process.platform === "darwin" ? "darwin" : "linux";
const arch = process.arch === "arm64" ? "arm64" : "amd64";
const exe = win ? "sing-box.exe" : "sing-box";

// curl honours the system proxy/IPv4 fallback better than Node's built-in fetch.
const curl_ = (u, dest) => execFileSync(win ? join(process.env.SystemRoot, "System32", "curl.exe") : "curl", ["-fL", "-sS", "--retry", "3", "-o", dest, u], { stdio: "inherit" });

const out = resolve("src-tauri", "binaries");
mkdirSync(out, { recursive: true });
if (existsSync(join(out, exe))) {
  console.log(`sing-box already present in ${out}`);
  process.exit(0);
}

const name = `sing-box-${VERSION}-${os}-${arch}`;
const url = `https://github.com/SagerNet/sing-box/releases/download/v${VERSION}/${name}.${ext}`;
const work = join(tmpdir(), `vanish-core-${Date.now()}`);
mkdirSync(work, { recursive: true });
const archive = join(work, `core.${ext}`);

console.log(`Downloading ${url}`);
curl_(url, archive);
execFileSync(win ? join(process.env.SystemRoot, "System32", "tar.exe") : "tar", ["-xf", archive, "-C", work], { stdio: "inherit" }); // bsdtar/GNU tar handle both formats

copyFileSync(join(work, name, exe), join(out, exe));
for (const extra of readdirSync(join(work, name)).filter((f) => f.endsWith(".dll") || f.endsWith(".so"))) {
  copyFileSync(join(work, name, extra), join(out, extra));
}
rmSync(work, { recursive: true, force: true });
console.log(`sing-box ${VERSION} -> ${out}`);
