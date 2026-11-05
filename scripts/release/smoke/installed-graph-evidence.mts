import type { Engine, FrozenReadContextV1, GraphEvidenceResolveRequestV1 } from "fathomdb";

function resolveInstalledEvidence(engine: Engine, context: FrozenReadContextV1) {
  const request: GraphEvidenceResolveRequestV1 = {
    schemaVersion: 1,
    evidenceRef: "installed-public-evidence-reference",
    context,
  };
  return engine.resolveGraphEvidence(request);
}

void resolveInstalledEvidence;
