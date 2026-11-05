PRAGMA temp_store=MEMORY;
CREATE TEMP TABLE IF NOT EXISTS poc_operation(claim TEXT PRIMARY KEY,body TEXT NOT NULL,source TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS poc_claims(claim TEXT PRIMARY KEY,source TEXT NOT NULL,kind TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS poc_cuts(cut INTEGER PRIMARY KEY);
INSERT OR IGNORE INTO poc_cuts VALUES(0);
CREATE TEMP TABLE IF NOT EXISTS poc_intent(claim TEXT PRIMARY KEY,v0 INTEGER NOT NULL,v1 INTEGER NOT NULL CHECK(v0<v1),withdrawn INTEGER NOT NULL CHECK(withdrawn IN (0,1)));
CREATE TABLE IF NOT EXISTS poc_fragments(fragment INTEGER PRIMARY KEY,claim TEXT NOT NULL,revision INTEGER NOT NULL,source TEXT NOT NULL,v0 INTEGER NOT NULL,v1 INTEGER NOT NULL CHECK(v0<v1),k0 INTEGER NOT NULL,k1 INTEGER CHECK(k1 IS NULL OR k0<k1),withdrawn INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS poc_claim_knowledge ON poc_fragments(claim,k1,v0,v1);
CREATE INDEX IF NOT EXISTS poc_revision ON poc_fragments(revision);
CREATE TEMP TRIGGER IF NOT EXISTS poc_temporal_insert AFTER INSERT ON main.canonical_nodes
WHEN EXISTS(SELECT 1 FROM poc_intent i JOIN poc_operation o ON i.claim=o.claim
WHERE i.claim=NEW.logical_id AND o.body=NEW.body AND o.source=NEW.source_id)
BEGIN
  SELECT CASE WHEN EXISTS(SELECT 1 FROM poc_claims WHERE claim=NEW.logical_id AND (source<>NEW.source_id OR kind<>NEW.kind)) THEN RAISE(ABORT,'claim source and kind are immutable') END;
  INSERT OR IGNORE INTO poc_claims VALUES(NEW.logical_id,NEW.source_id,NEW.kind);
  INSERT INTO poc_cuts VALUES(NEW.write_cursor);
  INSERT INTO poc_fragments(claim,revision,source,v0,v1,k0,withdrawn)
    SELECT f.claim,f.revision,f.source,f.v0,i.v0,NEW.write_cursor,f.withdrawn
    FROM poc_fragments f JOIN poc_intent i ON i.claim=f.claim
    WHERE f.claim=NEW.logical_id AND f.k1 IS NULL AND f.k0<NEW.write_cursor
      AND f.v0<i.v0 AND i.v0<f.v1;
  INSERT INTO poc_fragments(claim,revision,source,v0,v1,k0,withdrawn)
    SELECT f.claim,f.revision,f.source,i.v1,f.v1,NEW.write_cursor,f.withdrawn
    FROM poc_fragments f JOIN poc_intent i ON i.claim=f.claim
    WHERE f.claim=NEW.logical_id AND f.k1 IS NULL AND f.k0<NEW.write_cursor
      AND f.v0<i.v1 AND i.v1<f.v1;
  UPDATE poc_fragments SET k1=NEW.write_cursor
    WHERE claim=NEW.logical_id AND k1 IS NULL AND k0<NEW.write_cursor
      AND v0<(SELECT v1 FROM poc_intent WHERE claim=NEW.logical_id)
      AND (SELECT v0 FROM poc_intent WHERE claim=NEW.logical_id)<v1;
  INSERT INTO poc_fragments(claim,revision,source,v0,v1,k0,withdrawn)
    SELECT claim,NEW.write_cursor,NEW.source_id,v0,v1,NEW.write_cursor,withdrawn
    FROM poc_intent WHERE claim=NEW.logical_id;
  DELETE FROM poc_intent WHERE claim=NEW.logical_id;
  DELETE FROM poc_operation WHERE claim=NEW.logical_id;
END;
CREATE TRIGGER IF NOT EXISTS poc_temporal_erase AFTER DELETE ON canonical_nodes
BEGIN
DELETE FROM poc_fragments WHERE revision=OLD.write_cursor;
DELETE FROM poc_claims WHERE claim=OLD.logical_id AND NOT EXISTS(SELECT 1 FROM poc_fragments f WHERE f.claim=poc_claims.claim);
END;
