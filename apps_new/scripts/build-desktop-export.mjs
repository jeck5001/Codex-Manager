import { spawn } from "node:child_process";
import { access, copyFile, mkdir, readdir, rename, rm, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const desktopBuildCacheDir = path.join(projectRoot, ".desktop-build-cache");
const desktopWorkspaceDir = path.join(desktopBuildCacheDir, "workspace");
const desktopOutDir = path.join(projectRoot, "out");

const desktopBuildEntries = [
  "package.json",
  "next.config.ts",
  "tsconfig.json",
  "postcss.config.mjs",
  "components.json",
  "next-env.d.ts",
  "public",
  "src",
];

function logStep(message) {
  console.log(`[build:desktop] ${message}`);
}

async function pathExists(targetPath) {
  try {
    await access(targetPath);
    return true;
  } catch {
    return false;
  }
}

async function runCommand(command, args, cwd, env = {}) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env: {
        ...process.env,
        ...env,
      },
      shell: process.platform === "win32",
      stdio: "inherit",
    });

    child.on("error", reject);
    child.on("exit", (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(" ")} failed (code=${code}, signal=${signal ?? "none"})`));
    });
  });
}

function shouldSkipRelativePath(relativePath) {
  if (!relativePath) {
    return false;
  }

  const normalized = relativePath.split(path.sep).join("/");
  return (
    normalized === "src/app/api"
    || normalized.startsWith("src/app/api/")
    || normalized === ".next"
    || normalized.startsWith(".next/")
    || normalized === "out"
    || normalized.startsWith("out/")
    || normalized === ".desktop-build-cache"
    || normalized.startsWith(".desktop-build-cache/")
    || normalized === "node_modules"
    || normalized.startsWith("node_modules/")
  );
}

async function copyTree(sourcePath, destinationPath, relativePath = "") {
  if (shouldSkipRelativePath(relativePath)) {
    return;
  }

  const sourceStat = await stat(sourcePath);
  if (sourceStat.isDirectory()) {
    await mkdir(destinationPath, { recursive: true });
    const entries = await readdir(sourcePath, { withFileTypes: true });
    for (const entry of entries) {
      const childRelativePath = relativePath
        ? path.join(relativePath, entry.name)
        : entry.name;
      await copyTree(
        path.join(sourcePath, entry.name),
        path.join(destinationPath, entry.name),
        childRelativePath,
      );
    }
    return;
  }

  await mkdir(path.dirname(destinationPath), { recursive: true });
  await copyFile(sourcePath, destinationPath);
}

async function prepareDesktopWorkspace() {
  await rm(desktopBuildCacheDir, { recursive: true, force: true });
  await rm(desktopOutDir, { recursive: true, force: true });
  await mkdir(desktopWorkspaceDir, { recursive: true });

  for (const entry of desktopBuildEntries) {
    await copyTree(
      path.join(projectRoot, entry),
      path.join(desktopWorkspaceDir, entry),
      entry,
    );
  }
}

async function main() {
  logStep("creating isolated workspace for static desktop export");
  await prepareDesktopWorkspace();

  const pnpmCommand = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
  await runCommand(
    pnpmCommand,
    ["exec", "next", "build"],
    desktopWorkspaceDir,
    { NEXT_DESKTOP_EXPORT: "1" },
  );

  const exportedOutDir = path.join(desktopWorkspaceDir, "out");
  if (!(await pathExists(exportedOutDir))) {
    throw new Error("desktop export completed but no static output directory was found");
  }

  await rm(desktopOutDir, { recursive: true, force: true });
  await rename(exportedOutDir, desktopOutDir);
  await rm(desktopBuildCacheDir, { recursive: true, force: true });

  logStep("desktop static export is ready in out/");
}

main().catch(async (error) => {
  console.error(`[build:desktop] ${error instanceof Error ? error.message : String(error)}`);
  await rm(desktopBuildCacheDir, { recursive: true, force: true }).catch(() => {});
  process.exitCode = 1;
});
