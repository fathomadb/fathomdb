use std::sync::Arc;

use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::wal_attribution::{WalAttributionCollector, WalAttributionRole};

/// Begin a deferred reader transaction and attribute its real SQLite snapshot.
pub(crate) fn begin_attributed_reader_tx<'a>(
    reader: &'a mut Connection,
    attribution: &Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> rusqlite::Result<Transaction<'a>> {
    let tx = reader.transaction_with_behavior(TransactionBehavior::Deferred)?;
    if attribution.enabled {
        attribution.set(WalAttributionRole::ReaderWorker, worker_idx, true, "transaction_opened");
        tx.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, i64>(0))?;
        attribution.set(WalAttributionRole::ReaderWorker, worker_idx, true, "snapshot_acquired");
        #[cfg(any(test, feature = "test-hooks"))]
        attribution.fire_reader_snapshot_pause(&tx, worker_idx);
    }
    Ok(tx)
}
