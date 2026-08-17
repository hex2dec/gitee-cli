#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

const SUPPORTED_PLATFORMS = {
  "darwin-arm64": {
    packageName: "@pkg-in/gitee-cli-darwin-arm64",
    binaryPath: "bin/gitee"
  },
  "linux-x64": {
    packageName: "@pkg-in/gitee-cli-linux-x64",
    binaryPath: "bin/gitee"
  }
};

function supportedPlatformList() {
  return Object.keys(SUPPORTED_PLATFORMS).sort().join(", ");
}

function selectedPlatform() {
  return `${process.platform}-${process.arch}`;
}

function resolveBinary() {
  const platform = selectedPlatform();
  const selected = SUPPORTED_PLATFORMS[platform];

  if (!selected) {
    throw new Error(
      `Unsupported platform for @pkg-in/gitee-cli: ${platform}. ` +
        `Supported platforms: ${supportedPlatformList()}. ` +
        "Install a GitHub Release archive or build from source with cargo build --release."
    );
  }

  try {
    return require.resolve(`${selected.packageName}/${selected.binaryPath}`);
  } catch (error) {
    if (error && error.code === "MODULE_NOT_FOUND") {
      throw new Error(
        `Could not find ${selected.packageName}. ` +
          "Reinstall @pkg-in/gitee-cli so npm can install its optional platform dependency."
      );
    }

    throw error;
  }
}

function main() {
  let binary;

  try {
    binary = resolveBinary();
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }

  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

  if (result.error) {
    console.error(`Failed to start gitee binary: ${result.error.message}`);
    process.exit(1);
  }

  if (result.signal) {
    process.kill(process.pid, result.signal);
    return;
  }

  process.exit(result.status === null ? 1 : result.status);
}

main();
