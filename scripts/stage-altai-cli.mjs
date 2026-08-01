#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const tauriDir = join(repoRoot, "src-tauri");

function option(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}

const target =
  option("--target") ||
  process.env.TAURI_ENV_TARGET_TRIPLE ||
  execFileSync("rustc", ["--print", "host-tuple"], { encoding: "utf8" }).trim();
const profile = option("--profile") || "release";
const skipBuild = process.argv.includes("--skip-build");
const windows = target.includes("windows");
const binaryName = windows ? "altai-cli.exe" : "altai-cli";
const binary = option("--binary") || join(tauriDir, "target", target, profile, binaryName);
const staged = join(
  tauriDir,
  "binaries",
  `altai-cli-${target}${windows ? ".exe" : ""}`,
);

if (!skipBuild) {
  const cargoArgs = [
    "build",
    "--manifest-path",
    join(tauriDir, "Cargo.toml"),
    "-p",
    "altai-cli",
    "--target",
    target,
  ];
  if (profile === "release") cargoArgs.push("--release");
  else if (profile !== "debug") cargoArgs.push("--profile", profile);
  execFileSync("cargo", cargoArgs, { cwd: repoRoot, stdio: "inherit" });
}

if (!existsSync(binary)) {
  throw new Error(`CLI binary was not produced at ${binary}`);
}

mkdirSync(dirname(staged), { recursive: true });
copyFileSync(binary, staged);
if (!windows) chmodSync(staged, 0o755);
console.log(`staged ${staged}`);
