/* Run the current reader's exact SQL against owned copies using bundled SQLite. */
#define _POSIX_C_SOURCE 200809L

#include <sqlite3.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

static void check(sqlite3 *db, int status, int expected) {
    if (status != expected) {
        fprintf(stderr, "SQLite status %d: %s\n", status, sqlite3_errmsg(db));
        exit(EXIT_FAILURE);
    }
}

static void run(sqlite3 *db, const char *name, const char *sql) {
    sqlite3_stmt *statement;
    char *eqp = sqlite3_mprintf("EXPLAIN QUERY PLAN %s", sql);
    check(db, sqlite3_prepare_v2(db, eqp, -1, &statement, NULL), SQLITE_OK);
    sqlite3_free(eqp);
    int status;
    while ((status = sqlite3_step(statement)) == SQLITE_ROW) {
        printf("%s plan: %s\n", name, sqlite3_column_text(statement, 3));
    }
    check(db, status, SQLITE_DONE);
    check(db, sqlite3_finalize(statement), SQLITE_OK);

    check(db, sqlite3_prepare_v2(db, sql, -1, &statement, NULL), SQLITE_OK);
    if (sqlite3_bind_parameter_count(statement) == 4) {
        check(db, sqlite3_bind_int64(statement, 1, 1800000000), SQLITE_OK);
        for (int i = 2; i <= 4; i++) {
            check(db, sqlite3_bind_int(statement, i, 0), SQLITE_OK);
        }
    }
    struct timespec start;
    struct timespec end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    check(db, sqlite3_step(statement), SQLITE_ROW);
    int result = sqlite3_column_int(statement, 0);
    check(db, sqlite3_step(statement), SQLITE_DONE);
    clock_gettime(CLOCK_MONOTONIC, &end);
    printf("%s result=%d fullscan_steps=%d vm_steps=%d elapsed_ns=%lld\n",
           name, result,
           sqlite3_stmt_status(statement, SQLITE_STMTSTATUS_FULLSCAN_STEP, 0),
           sqlite3_stmt_status(statement, SQLITE_STMTSTATUS_VM_STEP, 0),
           (long long)(end.tv_sec - start.tv_sec) * 1000000000 +
               (end.tv_nsec - start.tv_nsec));
    check(db, sqlite3_finalize(statement), SQLITE_OK);
}

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "expected an owned store, dependency SQL and eligibility SQL\n");
        return EXIT_FAILURE;
    }
    sqlite3 *db = NULL;
    check(db, sqlite3_open_v2(argv[1], &db, SQLITE_OPEN_READWRITE, NULL), SQLITE_OK);
    printf("SQLite %s %s\n", sqlite3_libversion(), sqlite3_sourceid());
    check(db, sqlite3_exec(db, "BEGIN", NULL, NULL, NULL), SQLITE_OK);
    run(db, "dependency", argv[2]);
    run(db, "all-eligible", argv[3]);
    check(db, sqlite3_exec(db, "UPDATE canonical_nodes SET state='deleted' "
                              "WHERE rowid=(SELECT min(rowid) FROM canonical_nodes)",
                           NULL, NULL, NULL), SQLITE_OK);
    run(db, "early-ineligible", argv[3]);
    check(db, sqlite3_exec(db, "ROLLBACK; BEGIN;"
                              "UPDATE canonical_nodes SET valid_from=1800000001 "
                              "WHERE rowid=(SELECT min(rowid) FROM canonical_nodes "
                              "WHERE write_cursor>=(SELECT (min(write_cursor) + "
                              "max(write_cursor)) / 2 FROM canonical_nodes));"
                              "UPDATE canonical_nodes SET valid_until=1800000000 "
                              "WHERE rowid=(SELECT max(rowid) FROM canonical_nodes)",
                           NULL, NULL, NULL), SQLITE_OK);
    run(db, "mixed-window", argv[3]);
    check(db, sqlite3_exec(db, "ROLLBACK", NULL, NULL, NULL), SQLITE_OK);
    check(db, sqlite3_close(db), SQLITE_OK);
}
