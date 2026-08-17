#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const version = process.argv[2];

if (!version) {
  console.error("Usage: node scripts/set-root-cargo-version.mjs <version>");
  process.exit(2);
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const cargoTomlPath = path.resolve(scriptDir, "..", "Cargo.toml");
const original = fs.readFileSync(cargoTomlPath, "utf8");
const workspacePackageStart = original.indexOf("[workspace.package]");

if (workspacePackageStart === -1) {
  throw new Error("Cargo.toml does not contain a [workspace.package] section");
}

const nextSectionStart = original.indexOf("\n[", workspacePackageStart + "[workspace.package]".length);
const workspacePackageEnd = nextSectionStart === -1 ? original.length : nextSectionStart;
const before = original.slice(0, workspacePackageStart);
const workspacePackageSection = original.slice(workspacePackageStart, workspacePackageEnd);
const after = original.slice(workspacePackageEnd);
const versionPattern = /^version = "[^"]+"$/m;

if (!versionPattern.test(workspacePackageSection)) {
  throw new Error("Cargo.toml [workspace.package] section does not contain a version field");
}

const updatedWorkspacePackageSection = workspacePackageSection.replace(
  versionPattern,
  `version = "${version}"`,
);

fs.writeFileSync(cargoTomlPath, `${before}${updatedWorkspacePackageSection}${after}`);
