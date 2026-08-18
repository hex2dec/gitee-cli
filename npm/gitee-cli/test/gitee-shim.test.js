"use strict";

const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const test = require("node:test");

const packageRoot = path.resolve(__dirname, "..");
const shimSource = path.join(packageRoot, "bin", "gitee.js");

function makeTempPackage() {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "gitee-cli-shim-"));
  const appRoot = path.join(tempRoot, "app");
  fs.mkdirSync(path.join(appRoot, "bin"), { recursive: true });
  fs.copyFileSync(shimSource, path.join(appRoot, "bin", "gitee.js"));

  const preloadPath = path.join(tempRoot, "preload.js");
  fs.writeFileSync(
    preloadPath,
    [
      "const [platform, arch] = process.env.GITEE_CLI_TEST_PLATFORM.split('-');",
      "Object.defineProperty(process, 'platform', { value: platform });",
      "Object.defineProperty(process, 'arch', { value: arch });",
      ""
    ].join("\n")
  );

  return { tempRoot, appRoot, preloadPath };
}

function installFakePlatformPackage(appRoot, packageSuffix = "linux-x64") {
  const packageRoot = path.join(
    appRoot,
    "node_modules",
    "@pkg-ai",
    `gitee-cli-${packageSuffix}`
  );
  const binaryPath = path.join(packageRoot, "bin", "gitee");

  fs.mkdirSync(path.dirname(binaryPath), { recursive: true });
  fs.writeFileSync(
    binaryPath,
    [
      "#!/usr/bin/env node",
      "const fs = require('node:fs');",
      "if (process.env.GITEE_FAKE_ARGV_PATH) {",
      "  fs.writeFileSync(process.env.GITEE_FAKE_ARGV_PATH, JSON.stringify(process.argv.slice(2)));",
      "}",
      "if (process.env.GITEE_FAKE_MARKER_PATH) {",
      "  fs.writeFileSync(process.env.GITEE_FAKE_MARKER_PATH, 'ran');",
      "}",
      "process.exit(Number(process.env.GITEE_FAKE_EXIT_CODE || '0'));",
      ""
    ].join("\n")
  );
  fs.chmodSync(binaryPath, 0o755);
}

function runShim({ appRoot, preloadPath, platform = "linux-x64", args = [], env = {} }) {
  return spawnSync(
    process.execPath,
    ["--require", preloadPath, path.join(appRoot, "bin", "gitee.js"), ...args],
    {
      cwd: appRoot,
      env: {
        ...process.env,
        ...env,
        GITEE_CLI_TEST_PLATFORM: platform
      },
      encoding: "utf8"
    }
  );
}

test("shim forwards arguments to the selected platform binary", () => {
  const { tempRoot, appRoot, preloadPath } = makeTempPackage();
  try {
    installFakePlatformPackage(appRoot);
    const argvPath = path.join(tempRoot, "argv.json");

    const result = runShim({
      appRoot,
      preloadPath,
      args: ["pr", "list", "--json"],
      env: { GITEE_FAKE_ARGV_PATH: argvPath }
    });

    assert.equal(result.status, 0);
    assert.deepEqual(JSON.parse(fs.readFileSync(argvPath, "utf8")), [
      "pr",
      "list",
      "--json"
    ]);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("shim exits with the platform binary exit code", () => {
  const { tempRoot, appRoot, preloadPath } = makeTempPackage();
  try {
    installFakePlatformPackage(appRoot);

    const result = runShim({
      appRoot,
      preloadPath,
      env: { GITEE_FAKE_EXIT_CODE: "37" }
    });

    assert.equal(result.status, 37);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("unsupported platforms fail without falling back to PATH", () => {
  const { tempRoot, appRoot, preloadPath } = makeTempPackage();
  try {
    const pathFallbackDir = path.join(tempRoot, "path-bin");
    const markerPath = path.join(tempRoot, "path-fallback-ran");
    fs.mkdirSync(pathFallbackDir);
    const fallbackBinary = path.join(pathFallbackDir, "gitee");
    fs.writeFileSync(
      fallbackBinary,
      ["#!/usr/bin/env node", `require('node:fs').writeFileSync(${JSON.stringify(markerPath)}, 'ran');`, ""].join(
        "\n"
      )
    );
    fs.chmodSync(fallbackBinary, 0o755);

    const result = runShim({
      appRoot,
      preloadPath,
      platform: "win32-x64",
      env: { PATH: `${pathFallbackDir}${path.delimiter}${process.env.PATH}` }
    });

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /Unsupported platform/);
    assert.equal(fs.existsSync(markerPath), false);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
});
