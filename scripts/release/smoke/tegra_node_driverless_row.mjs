// One fresh Node process per row for smoke-tegra-node-driverless.sh.
//
// The probe copies this file into the consumer it installed, so the bare
// `fathomdb` import resolves through that consumer's node_modules exactly as
// an application's would. It records facts, not verdicts:
// tegra_node_driverless_verdict.mjs decides whether the row passed.
//
// argv: <row-name> <tegra-addon|loader> <out.json> <db-dir> <tegra-addon-path>
//   tegra-addon  load the installed Tegra `.node` directly and open and close
//                an engine without the default embedder;
//   loader       import the main package and record what its loader did.
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

const [name, kind, outFile, dbDir, addonPath] = process.argv.slice(2);
const require = createRequire(join(process.cwd(), "row.mjs"));
const out = {
  schema: "fathomdb.tegra-node-driverless-row/v1",
  name,
  kind,
  node: process.version,
  outcome: "error",
  error: null,
};

// The kernel's view of what the process mapped, not the loader's own report.
function mappedPaths() {
  const paths = new Set();
  for (const line of readFileSync("/proc/self/maps", "utf8").split("\n")) {
    const start = line.indexOf("/");
    if (start >= 0) paths.add(line.slice(start).trim());
  }
  return [...paths].sort();
}
const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

function recordMappings() {
  const mapped = mappedPaths();
  out.loaded_native_addons = mapped.filter((path) => path.endsWith(".node"));
  out.loaded_native_sha256 = Object.fromEntries(
    out.loaded_native_addons.map((path) => [path, sha256(path)]),
  );
  out.libcuda_mapped = mapped.some((path) => /\/libcuda\.so/.test(path));
}

try {
  if (kind === "tegra-addon") {
    out.early_init_env = process.env.FATHOMDB_CUDA_EARLY_INIT ?? null;
    out.opened = false;
    out.closed = false;
    const binding = require(addonPath);
    out.engine_open_type = typeof binding?.Engine?.open;
    const dir = mkdtempSync(join(dbDir, `${name}-`));
    const engine = await binding.Engine.open(join(dir, "driverless.fdb"), { useDefaultEmbedder: false });
    out.opened = true;
    await engine.close();
    out.closed = true;
    recordMappings();
    out.outcome = "completed";
  } else if (kind === "loader") {
    out.imported = false;
    out.error_name = null;
    out.error_message = null;
    try {
      await import("fathomdb");
      out.imported = true;
    } catch (error) {
      out.error_name = error?.name ?? null;
      out.error_message = error?.message ?? String(error);
    }
    recordMappings();
    out.outcome = "completed";
  } else {
    throw new Error(`unknown row kind: ${kind}`);
  }
} catch (error) {
  out.error = error?.stack ?? String(error);
  try {
    recordMappings();
  } catch {
    // The row is already a failure; keep the original error.
  }
}
writeFileSync(outFile, JSON.stringify(out, null, 2) + "\n");
process.exit(out.outcome === "completed" ? 0 : 1);
