"""Exercise a built candidate CLI's diagnostic contract without application writes."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def main() -> None:
    """Verify exact version, real integrity output, and immutable inspection."""
    binary, version = sys.argv[1:]
    binary = str(Path(binary).resolve(strict=True))

    def run(*args: str) -> str:
        result = subprocess.run([binary, *args], check=True, text=True, capture_output=True)
        print(result.stdout, end="")
        return result.stdout

    assert run("--version").strip() == f"fathomdb {version}"
    with tempfile.TemporaryDirectory(prefix="fathomdb-candidate-cli-") as directory:
        database = str(Path(directory) / "smoke.fdb")
        json.loads(run("doctor", "check-integrity", "--json", database))

        def snapshot() -> dict[str, str]:
            return {
                path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                for path in Path(directory).iterdir() if path.is_file()
            }

        before = snapshot()
        result = json.loads(run("doctor", "data-plane-integrity", "--json", database))
        assert result["schemaVersion"] == "fathomdb.doctor.data-plane-integrity.v1"
        assert result["status"] == "clean"
        assert before == snapshot(), "immutable inspection changed product files"
    print(f"candidate CLI {version}: exact identity and immutable integrity PASS")


if __name__ == "__main__":
    main()
