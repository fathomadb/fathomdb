import { Engine } from "<worktree>/src/ts/dist/index.js";
import { mkdtempSync } from "node:fs";
const d = mkdtempSync("<scratch>/db-");
try { const e = await Engine.open(d + "/t.fdb", { useDefaultEmbedder: true }); console.log("pass"); await e.close?.(); }
catch (err) { console.log("FAIL", err.kind ?? "", String(err.message).slice(0,200)); process.exitCode = 1; }
