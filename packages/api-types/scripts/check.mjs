import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../../..");
const committedSchema = join(repoRoot, "packages/api-types/src/schema.d.ts");
const openapiDocument = join(repoRoot, "openapi/openapi.yaml");
const generator = join(
  repoRoot,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "openapi-typescript.cmd" : "openapi-typescript",
);

const tempDir = mkdtempSync(join(tmpdir(), "kira-api-types-"));

try {
  const tempSchema = join(tempDir, "schema.d.ts");
  const result = spawnSync(
    generator,
    [openapiDocument, "-o", tempSchema],
    { cwd: repoRoot, stdio: "inherit" },
  );

  if (result.error) {
    console.error(`failed to run openapi-typescript: ${result.error.message}`);
    process.exitCode = 1;
  } else if (result.status !== 0) {
    console.error(`openapi-typescript exited with status ${result.status}`);
    process.exitCode = 1;
  } else if (!readFileSync(committedSchema).equals(readFileSync(tempSchema))) {
    console.error("schema.d.ts is stale; run npm run gen:api-types");
    process.exitCode = 1;
  }
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
