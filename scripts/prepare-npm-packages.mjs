#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const platforms = {
  "darwin-arm64": {
    directory: "gitee-cli-darwin-arm64",
    packageName: "@pkg-ai/gitee-cli-darwin-arm64"
  },
  "linux-x64": {
    directory: "gitee-cli-linux-x64",
    packageName: "@pkg-ai/gitee-cli-linux-x64"
  }
};

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

function parseArgs(argv) {
  const [kind, ...rest] = argv;
  const options = { kind };

  for (let i = 0; i < rest.length; i += 2) {
    const key = rest[i];
    const value = rest[i + 1];

    if (!key || !key.startsWith("--") || value === undefined) {
      usage();
    }

    options[key.slice(2)] = value;
  }

  return options;
}

function usage() {
  console.error(
    [
      "Usage:",
      "  node scripts/prepare-npm-packages.mjs main --version <version> --out <dir>",
      "  node scripts/prepare-npm-packages.mjs platform --platform <darwin-arm64|linux-x64> --version <version> --binary <path> --out <dir>"
    ].join("\n")
  );
  process.exit(2);
}

function requireOption(options, key) {
  const value = options[key];
  if (!value) {
    usage();
  }
  return value;
}

function copyFile(source, destination, mode) {
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.copyFileSync(source, destination);

  if (mode !== undefined) {
    fs.chmodSync(destination, mode);
  }
}

function readPackageJson(source) {
  return JSON.parse(fs.readFileSync(source, "utf8"));
}

function writePackageJson(destination, packageJson) {
  fs.writeFileSync(destination, `${JSON.stringify(packageJson, null, 2)}\n`);
}

function resetDirectory(directory) {
  fs.rmSync(directory, { recursive: true, force: true });
  fs.mkdirSync(directory, { recursive: true });
}

function prepareMainPackage({ version, out }) {
  const sourceDir = path.join(repoRoot, "npm", "gitee-cli");
  const destinationDir = path.join(out, "gitee-cli");
  resetDirectory(destinationDir);

  const packageJson = readPackageJson(path.join(sourceDir, "package.json"));
  packageJson.version = version;
  packageJson.optionalDependencies = Object.fromEntries(
    Object.values(platforms).map((platform) => [platform.packageName, version])
  );

  writePackageJson(path.join(destinationDir, "package.json"), packageJson);
  copyFile(path.join(sourceDir, "bin", "gitee.js"), path.join(destinationDir, "bin", "gitee.js"), 0o755);
  copyFile(path.join(repoRoot, "README.md"), path.join(destinationDir, "README.md"));
  copyFile(path.join(repoRoot, "LICENSE"), path.join(destinationDir, "LICENSE"));
}

function preparePlatformPackage({ platform, version, binary, out }) {
  const platformConfig = platforms[platform];
  if (!platformConfig) {
    usage();
  }

  const sourceDir = path.join(repoRoot, "npm", platformConfig.directory);
  const destinationDir = path.join(out, platformConfig.directory);
  resetDirectory(destinationDir);

  const packageJson = readPackageJson(path.join(sourceDir, "package.json"));
  packageJson.version = version;

  writePackageJson(path.join(destinationDir, "package.json"), packageJson);
  copyFile(path.join(sourceDir, "README.md"), path.join(destinationDir, "README.md"));
  copyFile(path.join(repoRoot, "LICENSE"), path.join(destinationDir, "LICENSE"));
  copyFile(path.resolve(repoRoot, binary), path.join(destinationDir, "bin", "gitee"), 0o755);
}

const options = parseArgs(process.argv.slice(2));
const version = requireOption(options, "version");
const out = path.resolve(repoRoot, requireOption(options, "out"));

if (options.kind === "main") {
  prepareMainPackage({ version, out });
} else if (options.kind === "platform") {
  preparePlatformPackage({
    platform: requireOption(options, "platform"),
    version,
    binary: requireOption(options, "binary"),
    out
  });
} else {
  usage();
}
