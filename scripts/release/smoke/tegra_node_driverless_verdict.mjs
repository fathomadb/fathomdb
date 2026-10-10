// Verdict for smoke-tegra-node-driverless.sh over its retained evidence.
//
// usage: node tegra_node_driverless_verdict.mjs <evidence-dir>
// Writes <evidence-dir>/summary.json with outcome PASS or FAIL and exits 0
// only on PASS. Every expected row must exist and meet its rule; a missing
// row or a host that could not give driverless, non-Tegra proof is a failure.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const evidence = process.argv[2];
if (!evidence) {
  console.error("usage: tegra_node_driverless_verdict.mjs <evidence-dir>");
  process.exit(2);
}

const GENERIC_PACKAGE = '"fathomdb-linux-arm64-gnu"';
const GENERIC_SENTINEL = "the generic linux-arm64-gnu stand-in was selected";
const JETSON_SELECTION = "The installed Jetson package";
const ADDON_ROWS = {
  "tegra-addon-early-init-default": null,
  "tegra-addon-early-init-off": "off",
};
const LOADER_ROWS = ["loader-generic-coinstalled", "loader-tegra-only"];

const results = {};
const record = (name, problems) => {
  results[name] = problems;
  for (const problem of problems) console.error(`FAIL  ${name}: ${problem}`);
  if (problems.length === 0) console.log(`PASS  ${name}`);
};
const readJson = (path) => JSON.parse(readFileSync(path, "utf8"));
const isHex = (value, length) => typeof value === "string" && new RegExp(`^[0-9a-f]{${length}}$`).test(value);

function checkContext() {
  const path = join(evidence, "context.json");
  if (!existsSync(path)) {
    record("context", ["context.json is missing"]);
    return null;
  }
  const context = readJson(path);
  const problems = [];
  if (!isHex(context.candidate_sha, 40)) problems.push(`candidate SHA ${context.candidate_sha} is not 40 lowercase hex`);
  for (const field of ["main_tarball_sha256", "tegra_tarball_sha256", "tegra_native_sha256"]) {
    if (!isHex(context[field], 64)) problems.push(`${field} is not a SHA-256`);
  }
  if (typeof context.tegra_native_path !== "string") problems.push("tegra_native_path is missing");
  record("context", problems);

  const host = context.host ?? {};
  const hostProblems = [];
  if (host.arch !== "aarch64") hostProblems.push(`architecture ${host.arch} is not aarch64`);
  if (host.tegra_family_signal !== false) hostProblems.push("Tegra-family signal is present or unrecorded");
  const present = (value) => !Array.isArray(value) || value.length > 0;
  if (present(host.libcuda)) hostProblems.push(`CUDA driver library is present or unrecorded: ${JSON.stringify(host.libcuda)}`);
  if (present(host.gpu_device_nodes)) {
    hostProblems.push(`GPU device node is present or unrecorded: ${JSON.stringify(host.gpu_device_nodes)}`);
  }
  if (present(host.nvidia_smi)) hostProblems.push(`nvidia-smi is present or unrecorded: ${JSON.stringify(host.nvidia_smi)}`);
  record("host", hostProblems);
  return problems.length === 0 ? context : null;
}

function loadRow(name) {
  const path = join(evidence, "rows", `${name}.json`);
  if (!existsSync(path)) {
    record(name, ["row is missing"]);
    return null;
  }
  return readJson(path);
}

function checkAddonRow(name, earlyInit, context) {
  const row = loadRow(name);
  if (row === null) return;
  const problems = [];
  if (row.outcome !== "completed" || row.error !== null) {
    problems.push(`outcome ${row.outcome}: ${row.error ?? "no error recorded"}`);
  }
  if (row.early_init_env !== earlyInit) {
    problems.push(`FATHOMDB_CUDA_EARLY_INIT was ${row.early_init_env}, expected ${earlyInit}`);
  }
  if (row.engine_open_type !== "function" || row.opened !== true || row.closed !== true) {
    problems.push("engine did not open and close");
  }
  const loaded = row.loaded_native_addons ?? [];
  if (context !== null) {
    if (JSON.stringify(loaded) !== JSON.stringify([context.tegra_native_path])) {
      problems.push(`loaded ${JSON.stringify(loaded)}, expected only ${context.tegra_native_path}`);
    } else if ((row.loaded_native_sha256 ?? {})[context.tegra_native_path] !== context.tegra_native_sha256) {
      problems.push("loaded addon SHA-256 differs from the packed Tegra binary");
    }
  }
  if (row.libcuda_mapped !== false) problems.push("libcuda was mapped or unrecorded");
  record(name, problems);
}

function checkLoaderRow(name) {
  const row = loadRow(name);
  if (row === null) return;
  const problems = [];
  const message = row.error_message ?? "";
  if (row.outcome !== "completed") problems.push(`outcome ${row.outcome}: ${row.error}`);
  if (row.imported !== false) problems.push("fathomdb imported, so a native addon other than the generic package served it");
  if ((row.loaded_native_addons ?? [null]).length !== 0) {
    problems.push(`a native addon was loaded: ${JSON.stringify(row.loaded_native_addons)}`);
  }
  if (message.includes(JETSON_SELECTION)) problems.push("the loader selected the Jetson package on a non-Tegra host");
  if (row.imported === false) {
    if (row.error_name !== "UnsupportedPlatformError") problems.push(`error ${row.error_name}, expected UnsupportedPlatformError`);
    if (!message.includes(GENERIC_PACKAGE)) problems.push("the loader error does not name the generic package");
    if (name === "loader-generic-coinstalled" && !message.includes(GENERIC_SENTINEL)) {
      problems.push("the generic stand-in was not selected");
    }
  }
  record(name, problems);
}

const context = checkContext();
for (const [name, earlyInit] of Object.entries(ADDON_ROWS)) checkAddonRow(name, earlyInit, context);
for (const name of LOADER_ROWS) checkLoaderRow(name);

const failed = Object.values(results).some((problems) => problems.length > 0);
writeFileSync(join(evidence, "summary.json"), JSON.stringify({
  schema: "fathomdb.tegra-node-driverless-summary/v1",
  outcome: failed ? "FAIL" : "PASS",
  candidate_sha: context?.candidate_sha ?? null,
  tegra_tarball_sha256: context?.tegra_tarball_sha256 ?? null,
  rows: results,
}, null, 2) + "\n");
console.log(`driverless Tegra Node verdict: ${failed ? "FAIL" : "PASS"}`);
process.exit(failed ? 1 : 0);
