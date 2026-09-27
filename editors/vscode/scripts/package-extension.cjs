const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const extensionRoot = path.resolve(__dirname, "..");
const workspaceRoot = path.resolve(extensionRoot, "../..");
const packageJson = require(path.join(extensionRoot, "package.json"));
const target = `${process.platform}-${process.arch}`;
const binarySuffix = process.platform === "win32" ? ".exe" : "";
const bundleDir = path.join(extensionRoot, "bin", target);
const vsixPath = path.join(
  extensionRoot,
  `${packageJson.name}-${packageJson.version}-${target}.vsix`
);

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}`);
  }
}

try {
  run("cargo", ["build", "--release", "--locked", "-p", "nexa-cli", "-p", "nexa-lsp"], workspaceRoot);

  fs.rmSync(bundleDir, { recursive: true, force: true });
  fs.mkdirSync(bundleDir, { recursive: true });
  for (const binary of ["nexa", "nexa-lsp"]) {
    const filename = `${binary}${binarySuffix}`;
    const bundledPath = path.join(bundleDir, filename);
    fs.copyFileSync(path.join(workspaceRoot, "target", "release", filename), bundledPath);
    if (process.platform !== "win32") fs.chmodSync(bundledPath, 0o755);
  }

  run("npm", ["run", "compile"], extensionRoot);
  const vsce = path.join(
    extensionRoot,
    "node_modules",
    ".bin",
    process.platform === "win32" ? "vsce.cmd" : "vsce"
  );
  run(vsce, ["package", "--target", target, "--out", vsixPath], extensionRoot);
  process.stdout.write(`\nCreated ${vsixPath}\n`);
} finally {
  fs.rmSync(bundleDir, { recursive: true, force: true });
}
