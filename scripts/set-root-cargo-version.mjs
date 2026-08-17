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
const packageStart = original.indexOf("[package]");

if (packageStart === -1) {
  throw new Error("Cargo.toml does not contain a [package] section");
}

const nextSectionStart = original.indexOf("\n[", packageStart + "[package]".length);
const packageEnd = nextSectionStart === -1 ? original.length : nextSectionStart;
const before = original.slice(0, packageStart);
const packageSection = original.slice(packageStart, packageEnd);
const after = original.slice(packageEnd);
const updatedPackageSection = packageSection.replace(/^version = "[^"]+"$/m, `version = "${version}"`);

if (updatedPackageSection === packageSection) {
  throw new Error("Cargo.toml [package] section does not contain a version field");
}

fs.writeFileSync(cargoTomlPath, `${before}${updatedPackageSection}${after}`);
