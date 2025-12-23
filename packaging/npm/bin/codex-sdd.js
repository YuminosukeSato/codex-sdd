#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const path = require("node:path");

function resolvePackageName() {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === "darwin" && arch === "arm64") return "@codex-sdd/darwin-arm64";
  if (platform === "darwin" && arch === "x64") return "@codex-sdd/darwin-x64";
  if (platform === "linux" && arch === "x64") return "@codex-sdd/linux-x64";
  if (platform === "linux" && arch === "arm64") return "@codex-sdd/linux-arm64";
  if (platform === "win32" && arch === "x64") return "@codex-sdd/win32-x64";
  return null;
}

function resolveBinary() {
  const pkg = resolvePackageName();
  if (!pkg) {
    return null;
  }
  try {
    const pkgJson = require.resolve(path.join(pkg, "package.json"));
    const pkgDir = path.dirname(pkgJson);
    const binPath = path.join(pkgDir, "bin", "codex-sdd");
    return binPath;
  } catch (err) {
    return null;
  }
}

const bin = resolveBinary();
if (!bin) {
  console.error("codex-sdd バイナリが見つかりません。対応するプラットフォームのパッケージを明示的にインストールしてください。");
  console.error("例: npm install -g @codex-sdd/darwin-arm64");
  process.exit(1);
}

const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
process.exit(result.status ?? 1);
