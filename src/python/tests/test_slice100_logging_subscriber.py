"""Real SQLite and installed-native checks for Python logger subscription."""

from __future__ import annotations

import logging
import gc
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import weakref
from pathlib import Path

from fathomdb import Engine, admin
from fathomdb.errors import ClosingError, InvalidArgumentError, OverloadedError


class _Records(logging.Handler):
    def __init__(self) -> None:
        super().__init__()
        self.records: list[logging.LogRecord] = []
        self.changed = threading.Condition()

    def emit(self, record: logging.LogRecord) -> None:
        with self.changed:
            self.records.append(record)
            self.changed.notify_all()

    def wait_for_phase(self, phase: str) -> list[logging.LogRecord]:
        with self.changed:
            observed = self.changed.wait_for(
                lambda: any(
                    getattr(record, "fathomdb", {}).get("phase") == phase
                    for record in self.records
                ),
                timeout=5,
            )
            if not observed:
                raise AssertionError(f"missing {phase} record: {self.records!r}")
            return list(self.records)


class LoggingSubscriberTests(unittest.TestCase):
    def test_real_write_delivers_structured_started_and_finished(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            logger = logging.Logger("fathomdb-s100", level=logging.DEBUG)
            handler = _Records()
            logger.addHandler(handler)
            try:
                engine.attach_logging_subscriber(logger)
                admin.configure(engine, name="slice100", body="{}")
                records = handler.wait_for_phase("finished")
                phases = [
                    getattr(record, "fathomdb", {}).get("phase")
                    for record in records
                ]
                self.assertIn("started", phases)
                self.assertLess(phases.index("started"), phases.index("finished"))
                for record in records:
                    self.assertIsInstance(record, logging.LogRecord)
                    self.assertIsInstance(getattr(record, "fathomdb"), dict)
            finally:
                engine.close()

    def test_old_heartbeat_argument_is_not_silently_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            try:
                with self.assertRaises(TypeError):
                    getattr(engine, "attach_logging_subscriber")(
                        logging.Logger("fathomdb-s100"), heartbeat_interval_ms=1
                    )
            finally:
                engine.close()

    def test_profile_and_failing_handler_do_not_change_write_result(self) -> None:
        class FailingOnce(_Records):
            def __init__(self) -> None:
                super().__init__()
                self.failed = False

            def emit(self, record: logging.LogRecord) -> None:
                if not self.failed:
                    self.failed = True
                    raise RuntimeError("handler failure")
                super().emit(record)

        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            logger = logging.Logger("fathomdb-s100", level=logging.DEBUG)
            handler = FailingOnce()
            logger.addHandler(handler)
            try:
                engine.set_profiling(enabled=True)
                engine.attach_logging_subscriber(logger)
                admin.configure(engine, name="slice100", body="{}")
                records = handler.wait_for_phase("finished")
                self.assertTrue(
                    any("profile_record" in getattr(record, "fathomdb", {}) for record in records)
                )
            finally:
                engine.close()

    def test_handler_database_reentry_is_rejected_without_recursion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            logger = logging.Logger("fathomdb-s100", level=logging.DEBUG)
            attempted = threading.Event()
            errors: list[Exception] = []

            class Reenter(logging.Handler):
                def emit(self, record: logging.LogRecord) -> None:
                    for operation in (
                        lambda: engine.search("missing"),
                        lambda: Engine.open(str(Path(directory) / "nested.sqlite")),
                        engine.close,
                    ):
                        try:
                            operation()
                        except Exception as error:
                            errors.append(error)
                    attempted.set()

            logger.addHandler(Reenter())
            try:
                engine.attach_logging_subscriber(logger)
                admin.configure(engine, name="slice100", body="{}")
                self.assertTrue(attempted.wait(5))
                self.assertEqual(len(errors), 3)
                self.assertTrue(all(isinstance(error, InvalidArgumentError) for error in errors))
                admin.configure(engine, name="slice100-again", body="{}")
            finally:
                engine.close()

    def test_replacement_close_and_caller_owned_logger_lifetime(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            logger = logging.Logger("fathomdb-s100-old", level=logging.DEBUG)
            old = _Records()
            logger.addHandler(old)
            engine.attach_logging_subscriber(logger)
            new_logger = logging.Logger("fathomdb-s100-new", level=logging.DEBUG)
            current = _Records()
            new_logger.addHandler(current)
            engine.attach_logging_subscriber(new_logger)
            admin.configure(engine, name="slice100", body="{}")
            current.wait_for_phase("finished")
            self.assertEqual(old.records, [])
            logger_ref = weakref.ref(new_logger)
            del new_logger
            gc.collect()
            self.assertIsNone(logger_ref())
            engine.close()
            with self.assertRaises(Exception):
                engine.attach_logging_subscriber(logger)

    def test_idle_logger_collection_stops_worker_before_engine_close(self) -> None:
        task_dir = Path("/proc/self/task")
        if not task_dir.is_dir():
            self.skipTest("native thread identity requires Linux procfs")

        def worker_count() -> int:
            return sum(
                (task / "comm").read_text().startswith("fathomdb-python")
                for task in task_dir.iterdir()
                if (task / "comm").exists()
            )

        def wait_for_count(expected: int) -> bool:
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                if worker_count() == expected:
                    return True
                time.sleep(0.05)
            return worker_count() == expected

        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            baseline = worker_count()
            logger = logging.Logger("fathomdb-s100-gc")
            engine.attach_logging_subscriber(logger)
            self.assertTrue(wait_for_count(baseline + 1))
            logger_ref = weakref.ref(logger)
            del logger
            gc.collect()
            self.assertIsNone(logger_ref())
            self.assertTrue(wait_for_count(baseline))
            admin.configure(engine, name="still-open-after-logger-gc", body="{}")
            engine.close()

    def test_blocked_handler_bounds_replacement_workers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            blocked = threading.Event()
            release = threading.Event()

            class Block(logging.Handler):
                def emit(self, record: logging.LogRecord) -> None:
                    blocked.set()
                    release.wait(5)

            logger = logging.Logger("fathomdb-s100-old", level=logging.DEBUG)
            logger.addHandler(Block())
            engine.attach_logging_subscriber(logger)
            admin.configure(engine, name="slice100", body="{}")
            self.assertTrue(blocked.wait(5))
            new_logger = logging.Logger("fathomdb-s100-new")
            engine.attach_logging_subscriber(new_logger)
            try:
                with self.assertRaises(OverloadedError):
                    engine.attach_logging_subscriber(logging.Logger("third"))
            finally:
                release.set()
                engine.close()

    def test_slow_handler_reports_loss_and_normal_records_resume(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            engine.set_profiling(enabled=True)
            blocked = threading.Event()
            release = threading.Event()

            class BlockThenRecord(_Records):
                def emit(self, record: logging.LogRecord) -> None:
                    if not blocked.is_set():
                        blocked.set()
                        release.wait(30)
                    super().emit(record)

            logger = logging.Logger("fathomdb-s100-overflow", level=logging.DEBUG)
            handler = BlockThenRecord()
            logger.addHandler(handler)
            engine.attach_logging_subscriber(logger)
            try:
                admin.configure(engine, name="slice100-start", body="{}")
                self.assertTrue(blocked.wait(5))
                for index in range(1600):
                    admin.configure(engine, name=f"slice100-{index}", body="{}")
            finally:
                release.set()
            with handler.changed:
                self.assertTrue(
                    handler.changed.wait_for(
                        lambda: any(
                            getattr(record, "fathomdb", {}).get("dropped_records", 0) > 0
                            for record in handler.records
                        ),
                        timeout=15,
                    )
                )
                dropped_index = next(
                    index
                    for index, record in enumerate(handler.records)
                    if getattr(record, "fathomdb", {}).get("dropped_records", 0) > 0
                )
                self.assertTrue(
                    handler.changed.wait_for(
                        lambda: any(
                            "phase" in getattr(record, "fathomdb", {})
                            for record in handler.records[dropped_index + 1 :]
                        ),
                        timeout=15,
                    )
                )
            engine.close()

    def test_concurrent_attach_close_finishes_and_cannot_resurrect_logger(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            engine = Engine.open(str(Path(directory) / "subscriber.sqlite"))
            logger = logging.Logger("fathomdb-s100-race")
            start = threading.Barrier(3)
            errors: list[Exception] = []

            def attach() -> None:
                start.wait()
                try:
                    engine.attach_logging_subscriber(logger)
                except ClosingError:
                    pass
                except Exception as error:
                    errors.append(error)

            def close() -> None:
                start.wait()
                try:
                    engine.close()
                except Exception as error:
                    errors.append(error)

            threads = [threading.Thread(target=attach), threading.Thread(target=close)]
            for thread in threads:
                thread.start()
            start.wait()
            for thread in threads:
                thread.join(timeout=5)
                self.assertFalse(thread.is_alive())
            self.assertEqual(errors, [])
            with self.assertRaises(ClosingError):
                engine.attach_logging_subscriber(logger)

    def test_process_exits_with_idle_delivery_worker(self) -> None:
        script = """
import logging
import tempfile
from pathlib import Path
from fathomdb import Engine
with tempfile.TemporaryDirectory() as directory:
    engine = Engine.open(str(Path(directory) / 'child.sqlite'))
    logger = logging.Logger('fathomdb-s100-child')
    engine.attach_logging_subscriber(logger)
"""
        result = subprocess.run(
            [sys.executable, "-I", "-c", script],
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
