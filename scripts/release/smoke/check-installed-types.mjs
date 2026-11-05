import { createHash } from "node:crypto";
import { copyFileSync, lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const [consumerArg, compilerArg, mainArchive, platformArchive] = process.argv.slice(2);
if (!consumerArg || !compilerArg || !mainArchive || !platformArchive) {
  throw new Error("usage: check-installed-types.mjs <consumer> <tsc> <main.tgz> <platform.tgz>");
}
const consumer = realpathSync(consumerArg);
const compiler = realpathSync(compilerArg);
const installed = realpathSync(join(consumer, "node_modules/fathomdb"));
if (!installed.startsWith(consumer + sep)) throw new Error("installed type package escaped consumer");
const fixture = "installed-graph-evidence.mts";
copyFileSync(join(dirname(fileURLToPath(import.meta.url)), fixture), join(consumer, fixture));
const result = spawnSync(process.execPath, [
  compiler, "--strict", "--noEmit", "--module", "NodeNext", "--moduleResolution", "NodeNext",
  "--target", "ES2022", "--types", "node", "--typeRoots", resolve(dirname(compiler), "../../@types"), fixture,
], { cwd: consumer, stdio: "inherit" });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");
const declarations = [];
function visit(directory) {
  for (const name of readdirSync(directory).sort()) {
    const path = join(directory, name);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink()) throw new Error("symlinked installed declaration tree");
    if (stat.isDirectory()) visit(path);
    else if (name.endsWith(".d.ts")) declarations.push(`${relative(installed, path).split(sep).join("/")}\t${sha256(path)}`);
  }
}
visit(join(installed, "dist"));
console.log(JSON.stringify({
  proof: "installed-public-graph-types",
  main_package_sha256: sha256(mainArchive),
  platform_package_sha256: sha256(platformArchive),
  package_json_sha256: sha256(join(installed, "package.json")),
  declarations_sha256: createHash("sha256").update(declarations.sort().join("\n") + "\n").digest("hex"),
  compiler_version: JSON.parse(readFileSync(resolve(dirname(compiler), "../package.json"), "utf8")).version,
}));
