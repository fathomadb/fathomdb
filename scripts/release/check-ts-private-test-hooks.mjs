#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import ts from "../../src/ts/node_modules/typescript/lib/typescript.js";

function privateHookName(node) {
  if (ts.isPropertyAccessExpression(node)) {
    return node.name.text;
  }
  if (
    ts.isElementAccessExpression(node) &&
    (ts.isStringLiteral(node.argumentExpression) ||
      ts.isNoSubstitutionTemplateLiteral(node.argumentExpression))
  ) {
    return node.argumentExpression.text;
  }
  return undefined;
}

function findings(file) {
  const source = fs.readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const found = [];

  function visit(node) {
    const name = privateHookName(node);
    if (name?.endsWith("ForTest")) {
      const position = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
      found.push(`${file}:${position.line + 1}:${position.character + 1}: ${name}`);
    }
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return found;
}

const files = process.argv.slice(2);
if (files.length === 0) {
  console.error(`usage: ${path.basename(process.argv[1])} <typescript-file> [...]`);
  process.exit(2);
}

const violations = files.flatMap(findings);
if (violations.length > 0) {
  console.error("release-artifact module depends on a private test hook:");
  console.error(violations.join("\n"));
  process.exit(1);
}
