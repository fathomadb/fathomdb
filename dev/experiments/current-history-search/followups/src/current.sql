CREATE TABLE IF NOT EXISTS poc_certificate(singleton INTEGER PRIMARY KEY CHECK(singleton=1),published_at INTEGER NOT NULL,next_boundary INTEGER NOT NULL,visibility INTEGER NOT NULL,scope TEXT NOT NULL);
INSERT OR IGNORE INTO poc_certificate VALUES(1,-9223372036854775808,-9223372036854775808,-1,'strict-v1');
CREATE TABLE IF NOT EXISTS poc_dirty(cursor INTEGER PRIMARY KEY);
CREATE TABLE IF NOT EXISTS poc_frontier(at INTEGER NOT NULL,cursor INTEGER NOT NULL,PRIMARY KEY(at,cursor));
CREATE VIRTUAL TABLE IF NOT EXISTS poc_current USING fts5(body, tokenize='porter unicode61 remove_diacritics 2');
CREATE TRIGGER IF NOT EXISTS poc_dirty_insert AFTER INSERT ON canonical_nodes BEGIN INSERT OR IGNORE INTO poc_dirty VALUES(NEW.write_cursor); END;
CREATE TRIGGER IF NOT EXISTS poc_dirty_update AFTER UPDATE ON canonical_nodes BEGIN INSERT OR IGNORE INTO poc_dirty VALUES(NEW.write_cursor); END;
CREATE TRIGGER IF NOT EXISTS poc_dirty_delete AFTER DELETE ON canonical_nodes BEGIN INSERT OR IGNORE INTO poc_dirty VALUES(OLD.write_cursor); END;

CREATE TRIGGER IF NOT EXISTS poc_frontier_insert AFTER INSERT ON canonical_nodes BEGIN
 INSERT OR IGNORE INTO poc_frontier SELECT NEW.valid_from,NEW.write_cursor WHERE NEW.valid_from IS NOT NULL;
 INSERT OR IGNORE INTO poc_frontier SELECT NEW.valid_until,NEW.write_cursor WHERE NEW.valid_until IS NOT NULL;
END;
CREATE TRIGGER IF NOT EXISTS poc_frontier_update AFTER UPDATE ON canonical_nodes BEGIN
 DELETE FROM poc_frontier WHERE cursor=OLD.write_cursor;
 INSERT OR IGNORE INTO poc_frontier SELECT NEW.valid_from,NEW.write_cursor WHERE NEW.valid_from IS NOT NULL;
 INSERT OR IGNORE INTO poc_frontier SELECT NEW.valid_until,NEW.write_cursor WHERE NEW.valid_until IS NOT NULL;
END;
CREATE TRIGGER IF NOT EXISTS poc_frontier_delete AFTER DELETE ON canonical_nodes BEGIN DELETE FROM poc_frontier WHERE cursor=OLD.write_cursor; END;
CREATE INDEX IF NOT EXISTS poc_frontier_owner ON poc_frontier(cursor);

CREATE TRIGGER IF NOT EXISTS poc_link_insert AFTER INSERT ON _fathomdb_source_links BEGIN INSERT OR IGNORE INTO poc_dirty SELECT write_cursor FROM _fathomdb_artifact_revisions WHERE revision_id=NEW.artifact_revision_id AND artifact_class='node'; END;
CREATE TRIGGER IF NOT EXISTS poc_link_delete AFTER DELETE ON _fathomdb_source_links BEGIN INSERT OR IGNORE INTO poc_dirty SELECT write_cursor FROM _fathomdb_artifact_revisions WHERE revision_id=OLD.artifact_revision_id AND artifact_class='node'; END;
CREATE TRIGGER IF NOT EXISTS poc_closure_insert AFTER INSERT ON _fathomdb_dependency_closures BEGIN
 INSERT OR IGNORE INTO poc_dirty SELECT ar.write_cursor FROM _fathomdb_source_links link JOIN _fathomdb_artifact_revisions ar ON ar.revision_id=link.artifact_revision_id WHERE ar.artifact_class='node' AND ((NEW.root_kind='source_revision' AND link.source_revision_id=NEW.root_value) OR (NEW.root_kind='source_bucket' AND link.source_id=NEW.root_value)); END;
CREATE TRIGGER IF NOT EXISTS poc_closure_update AFTER UPDATE ON _fathomdb_dependency_closures BEGIN
 INSERT OR IGNORE INTO poc_dirty SELECT ar.write_cursor FROM _fathomdb_source_links link JOIN _fathomdb_artifact_revisions ar ON ar.revision_id=link.artifact_revision_id WHERE ar.artifact_class='node' AND ((NEW.root_kind='source_revision' AND link.source_revision_id=NEW.root_value) OR (NEW.root_kind='source_bucket' AND link.source_id=NEW.root_value)); END;
CREATE TRIGGER IF NOT EXISTS poc_closure_delete AFTER DELETE ON _fathomdb_dependency_closures BEGIN
 INSERT OR IGNORE INTO poc_dirty SELECT ar.write_cursor FROM _fathomdb_source_links link JOIN _fathomdb_artifact_revisions ar ON ar.revision_id=link.artifact_revision_id WHERE ar.artifact_class='node' AND ((OLD.root_kind='source_revision' AND link.source_revision_id=OLD.root_value) OR (OLD.root_kind='source_bucket' AND link.source_id=OLD.root_value)); END;
