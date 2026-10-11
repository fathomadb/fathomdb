import {
  Engine,
  admin,
  graph,
  read,
  rerank,
  embedBatchCls,
  type EngineConfig,
  type GraphExpandRequestV1,
  type SearchResult,
  type WriteReceipt,
  type EvidenceSearchResultV1,
  type FrozenReadContextV1,
  type ProjectionSpec,
} from "fathomdb";

const config: EngineConfig = { schedulerRuntimeThreads: 1 };
const frozen = null as unknown as FrozenReadContextV1;
const opened: Promise<Engine> = Engine.open("consumer.db", { engineConfig: config });
const written: Promise<WriteReceipt> = opened.then((engine) => engine.write([]));
const searched: Promise<SearchResult> = opened.then((engine) => engine.search("term"));
const evidence: Promise<EvidenceSearchResultV1> = opened.then((engine) =>
  engine.searchWithEvidence({
    schemaVersion: 1,
    query: "term",
    context: frozen,
  }),
);
const projection = null as unknown as ProjectionSpec;
const graphRequest = null as unknown as GraphExpandRequestV1;
void [admin, graph, read, rerank, embedBatchCls, written, searched, evidence, projection, graphRequest];
