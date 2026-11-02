CREATE TABLE IF NOT EXISTS poc_certificate(singleton INTEGER PRIMARY KEY CHECK(singleton=1),published_at INTEGER NOT NULL,next_boundary INTEGER NOT NULL,visibility INTEGER NOT NULL,scope TEXT NOT NULL);
INSERT OR IGNORE INTO poc_certificate VALUES(1,-9223372036854775808,-9223372036854775808,-1,'strict-v1');
CREATE TABLE IF NOT EXISTS poc_dirty(cursor INTEGER PRIMARY KEY);
CREATE TABLE IF NOT EXISTS poc_frontier(at INTEGER NOT NULL,cursor INTEGER NOT NULL,PRIMARY KEY(at,cursor));
CREATE VIRTUAL TABLE IF NOT EXISTS poc_current USING fts5(body, tokenize='porter unicode61 remove_diacritics 2');
CREATE TRIGGER IF NOT EXISTS poc_dirty_insert AFTER INSERT ON canonical_nodes BEGIN INSERT OR IGNORE INTO poc_dirty VALUES(NEW.write_cursor); END;
CREATE TRIGGER IF NOT EXISTS poc_dirty_update AFTER UPDATE ON canonical_nodes BEGIN INSERT OR IGNORE INTO poc_dirty VALUES(NEW.write_cursor); END;
CREATE TRIGGER IF NOT EXISTS poc_dirty_delete AFTER DELETE ON canonical_nodes BEGIN INSERT OR IGNORE INTO poc_dirty VALUES(OLD.write_cursor); END;
