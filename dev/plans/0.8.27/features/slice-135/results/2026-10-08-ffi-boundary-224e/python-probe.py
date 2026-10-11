import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory
import fathomdb
from fathomdb import Engine, admin
from fathomdb.errors import WriteValidationError

with TemporaryDirectory(prefix='slice135-python-ffi-224e-') as directory:
    path = str(Path(directory) / 'ffi.sqlite')
    engine = Engine.open(path)
    admin.configure(engine, name='ffi_col', body='{}')
    before = engine.counters().write_rows
    errors = []
    for body in ('{"x":"a\x00b"}', '{"x":"a\ud800b"}'):
        try:
            engine.write([{'op_store': {'collection': 'ffi_col', 'record_key': 'k1', 'body': body}}])
        except WriteValidationError as error:
            errors.append(type(error).__name__)
        else:
            raise AssertionError('invalid FFI body was accepted')
    after = engine.counters().write_rows
    assert before == after
    engine.close()
    reopened = Engine.open(path)
    reopened_rows = reopened.counters().write_rows
    assert reopened_rows == before
    reopened.close()
    print(json.dumps({'source_sha':'224e44c593c13d86ece648adabe445723db04070','wheel_sha256':'ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a','module_path':fathomdb.__file__,'pid':os.getpid(),'errors':errors,'before_write_rows':before,'after_write_rows':after,'reopened_write_rows':reopened_rows,'status':'PASS'}))
