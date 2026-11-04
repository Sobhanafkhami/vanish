// Downloads the official cores into src-tauri/binaries so they get bundled with the app:
//   sing-box  - the main engine
//   xray      - runs the transports sing-box has no client for (XHTTP / SplitHTTP, mKCP, TCP HTTP-header, VLESS encryption)
// `node scripts/fetch-core.mjs android` instead drops the arm64 Xray executable into the Android project's jniLibs
// as libxray.so; Android only lets an app execute files from its native library directory.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, copyFileSync, readdirSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const SING_BOX = "1.14.3";
const XRAY = "26.10.31";
const win = process.platform === "win32";
const ext = win ? "zip" : "tar.gz";
const os = win ? "windows" : process.platform === "darwin" ? "darwin" : "linux";
const arch = process.arch === "arm64" ? "arm64" : "amd64";

// curl honours the system proxy/IPv4 fallback better than Node's built-in fetch.
const curl_ = (u, dest) => execFileSync(win ? join(process.env.SystemRoot, "System32", "curl.exe") : "curl", ["-fL", "-sS", "--retry", "3", "-o", dest, u], { stdio: "inherit" });
const untar = (archive, dir) => execFileSync(win ? join(process.env.SystemRoot, "System32", "tar.exe") : "tar", ["-xf", archive, "-C", dir], { stdio: "inherit" }); // bsdtar/GNU tar handle zip and tar.gz
const scratch = () => {
  const dir = join(tmpdir(), `vanish-core-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`);
  mkdirSync(dir, { recursive: true });
  return dir;
};

function singBox(out) {
  const exe = win ? "sing-box.exe" : "sing-box";
  if (existsSync(join(out, exe))) return console.log(`sing-box already present in ${out}`);
  const name = `sing-box-${SING_BOX}-${os}-${arch}`;
  const url = `https://github.com/SagerNet/sing-box/releases/download/v${SING_BOX}/${name}.${ext}`;
  const work = scratch();
  console.log(`Downloading ${url}`);
  curl_(url, join(work, `core.${ext}`));
  untar(join(work, `core.${ext}`), work);
  copyFileSync(join(work, name, exe), join(out, exe));
  for (const extra of readdirSync(join(work, name)).filter((f) => f.endsWith(".dll") || f.endsWith(".so"))) {
    copyFileSync(join(work, name, extra), join(out, extra));
  }
  rmSync(work, { recursive: true, force: true });
  console.log(`sing-box ${SING_BOX} -> ${out}`);
}

function xray(asset, from, to) {
  if (existsSync(to)) return console.log(`xray already present: ${to}`);
  const url = `https://github.com/XTLS/Xray-core/releases/download/v${XRAY}/${asset}.zip`;
  const work = scratch();
  console.log(`Downloading ${url}`);
  curl_(url, join(work, "xray.zip"));
  untar(join(work, "xray.zip"), work);
  mkdirSync(resolve(to, ".."), { recursive: true });
  copyFileSync(join(work, from), to);
  rmSync(work, { recursive: true, force: true });
  console.log(`xray ${XRAY} -> ${to}`);
}

if (process.argv[2] === "android") {
  xray("Xray-android-arm64-v8a", "xray", resolve("src-tauri", "gen", "android", "app", "src", "main", "jniLibs", "arm64-v8a", "libxray.so"));
} else {
  const out = resolve("src-tauri", "binaries");
  mkdirSync(out, { recursive: true });
  singBox(out);
  const xrayAsset = win ? "Xray-windows-64" : os === "darwin" ? (arch === "arm64" ? "Xray-macos-arm64-v8a" : "Xray-macos-64") : arch === "arm64" ? "Xray-linux-arm64-v8a" : "Xray-linux-64";
  const xrayExe = win ? "xray.exe" : "xray";
  xray(xrayAsset, xrayExe, join(out, xrayExe));
}
