#!/usr/bin/env python3
"""Exact local fork consumption and refusal boundaries from Slice110's design."""

from pathlib import Path
import os
import importlib.util
import json
import tomllib
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CUDARC_REV = "343b04fa73f85ee78754d6ee2baa07b959cc9ccc"
CANDLE_REV = "2ee39305a78ed1463ff913f71dcd3d45ecbfd84f"
CUDARC_GIT = "file:///home/coreyt/projects/fathomdb-forks/cudarc"
CANDLE_GIT = "file:///home/coreyt/projects/fathomdb-forks/candle-fathomdb"
K9 = "182ceab0547348993b6dbec4b1aed610dcc8b23f"


class QualifiedForkPins(unittest.TestCase):
    """Require the accepted local graph while refusing unqualified replacements."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for relative in (
            "Cargo.toml", "Cargo.lock", "package.json", "package-lock.json",
            "scripts/pinned-override-rot.json", "scripts/pinned-override-advisories.json",
            "src/rust/crates/fathomdb-embedder/Cargo.toml",
        ):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)

        config = ROOT / ".cargo/config.toml"
        if config.is_file():
            (self.root / ".cargo").mkdir()
            shutil.copyfile(config, self.root / ".cargo/config.toml")

    def replace(self, relative, before, after):
        path = self.root / relative
        text = path.read_text()
        self.assertIn(before, text)
        path.write_text(text.replace(before, after))

    def check(self):
        return subprocess.run(
            ["python3", str(ROOT / "scripts/check-pinned-override-rot.py"), "--root", str(self.root)],
            capture_output=True, text=True, check=False,
        )

    def refused(self):
        result = self.check()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)

    def test_exact_qualified_graph_passes(self):
        result = self.check()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("local qualification", result.stdout)
        self.assertIn("publication", result.stdout)

    def test_source_identity_is_exact_for_each_fork(self):
        original = (self.root / "Cargo.lock").read_text()
        for url, revision in ((CUDARC_GIT, CUDARC_REV), (CANDLE_GIT, CANDLE_REV)):
            for before, after in (
                (url, url + "-other"),
                (url, url.replace("file:///home/coreyt/projects/fathomdb-forks/", "https://github.com/coreyt/") + ".git"),
                (revision, "0" * 40),
            ):
                with self.subTest(source=after):
                    (self.root / "Cargo.lock").write_text(original.replace(before, after))
                    self.refused()
        (self.root / "Cargo.lock").write_text(original)

    def test_unpublished_versions_cannot_claim_registry_checksums(self):
        self.replace("Cargo.lock", f"git+{CUDARC_GIT}?rev={CUDARC_REV}#{CUDARC_REV}",
                     "registry+https://github.com/rust-lang/crates.io-index")
        self.refused()

    def test_git_package_cannot_claim_a_published_checksum(self):
        source = f'source = "git+{CUDARC_GIT}?rev={CUDARC_REV}#{CUDARC_REV}"'
        self.replace("Cargo.lock", source, source + '\nchecksum = "' + "0" * 64 + '"')
        self.refused()

    def test_candle_family_cannot_mix_qualified_and_published_members(self):
        self.replace("Cargo.lock", 'name = "candle-nn-fathomdb"\nversion = "0.12.0"',
                     'name = "candle-nn-fathomdb"\nversion = "0.11.0"')
        self.refused()

    def test_manifest_cannot_downgrade_the_selected_family(self):
        self.replace("src/rust/crates/fathomdb-embedder/Cargo.toml", '=0.12.0', '=0.11.0')
        self.refused()

    def test_kernels_cannot_follow_the_new_local_candle_source(self):
        self.replace("Cargo.lock", f"git+https://github.com/coreyt/candle-fathomdb.git?rev={K9}#{K9}",
                     f"git+{CANDLE_GIT}?rev={CANDLE_REV}#{CANDLE_REV}")
        self.refused()

    def test_lock_drift_refresh_names_the_adopted_native_local_protocol(self):
        fixture = ROOT / "scripts/tests/fixtures/cudarc-lock-set/changed-libloading"
        result = subprocess.run(
            ["python3", str(ROOT / "scripts/check-cudarc-fathomdb-lock-set.py"),
             "--metadata", str(fixture / "metadata.json"), "--lock", str(fixture / "Cargo.lock"),
             "--record", str(fixture / "record.json")],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("both native Linux targets", result.stderr)
        self.assertIn("local protocol", result.stderr)

    def test_single_cudarc_accepts_only_the_selected_tree_source(self):
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text('#!/bin/sh\ncase " $* " in *" -e normal "*) printf "%s\\n" "$TEST_CUDARC_LINE" ;; esac\n')
        cargo.chmod(0o755)
        expected = f"cudarc-fathomdb v0.20.0 ({CUDARC_GIT}?rev={CUDARC_REV}#{CUDARC_REV[:8]})"
        for line, code in ((expected, 0), ("cudarc-fathomdb v0.20.0", 1),
                           (expected.replace(CUDARC_REV, "0" * 40), 1),
                           (expected.replace(CUDARC_GIT, CUDARC_GIT + "-other"), 1),
                           ("cudarc v0.20.0\n" + expected, 1)):
            with self.subTest(line=line):
                result = subprocess.run(
                    ["bash", str(ROOT / "scripts/check-single-cudarc.sh"), "--root", str(self.root)],
                    env={**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                         "TEST_CUDARC_LINE": line},
                    capture_output=True, text=True, check=False,
                )
                self.assertEqual(result.returncode, code, result.stdout + result.stderr)


MIRRORS = {
    "qualified-cudarc": {"git": CUDARC_GIT, "rev": CUDARC_REV, "replace-with": "cudarc-https"},
    "cudarc-https": {"git": "https://github.com/coreyt/cudarc.git", "rev": CUDARC_REV},
    "qualified-candle": {"git": CANDLE_GIT, "rev": CANDLE_REV, "replace-with": "candle-https"},
    "candle-https": {"git": "https://github.com/coreyt/candle-fathomdb.git", "rev": CANDLE_REV},
}


def portable_patches():
    return (
        f'cudarc-fathomdb = {{ git = "{CUDARC_GIT}", rev = "{CUDARC_REV}" }}\n'
        + "".join(f'{name}-fathomdb = {{ git = "{CANDLE_GIT}", rev = "{CANDLE_REV}" }}\n'
                  for name in ("candle-core", "candle-nn", "candle-transformers"))
        + f'\n[patch."{CANDLE_GIT}"]\ncandle-kernels = {{ git = "https://github.com/coreyt/candle-fathomdb.git", rev = "{K9}" }}\n'
    )


def write_mirrors(root, mirrors):
    target = root / ".cargo/config.toml"
    target.parent.mkdir(exist_ok=True)
    target.write_text("".join(f'[source.{name}]\n' + "".join(f'{key} = "{value}"\n' for key, value in spec.items())
                              + "\n" for name, spec in mirrors.items()))


class PortableForkConsumption(unittest.TestCase):
    """The source mirror is authenticated independently of the logical lock IDs."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        spec = importlib.util.spec_from_file_location("pin_checker", ROOT / "scripts/check-pinned-override-rot.py")
        self.checker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.checker)

    def test_exact_source_only_mirrors_are_allowed(self):
        write_mirrors(self.root, MIRRORS)
        self.assertEqual(self.checker.cargo_config_failures(self.root), [])

    def test_changed_physical_mirror_cannot_hide_behind_logical_lock_identity(self):
        for table, field, value in (
            ("candle-https", "rev", "0" * 40),
            ("cudarc-https", "git", "https://example.invalid/cudarc.git"),
            ("qualified-candle", "replace-with", "cudarc-https"),
            ("qualified-cudarc", "branch", "main"),
        ):
            with self.subTest(table=table, field=field):
                mirrors = {name: dict(spec) for name, spec in MIRRORS.items()}
                mirrors[table][field] = value
                write_mirrors(self.root, mirrors)
                self.assertTrue(self.checker.cargo_config_failures(self.root))

    def test_other_source_replacements_are_refused(self):
        mirrors = dict(MIRRORS)
        mirrors["crates-io"] = {"replace-with": "candle-https"}
        write_mirrors(self.root, mirrors)
        self.assertTrue(self.checker.cargo_config_failures(self.root))

    def test_repository_contains_the_exact_source_only_mirrors(self):
        path = ROOT / ".cargo/config.toml"
        self.assertTrue(path.is_file(), "the portable repository must carry its source mirrors")
        self.assertEqual(tomllib.loads(path.read_text()), {"source": MIRRORS})

    def test_explicit_portable_patch_cohort_passes(self):
        fixture = QualifiedForkPins(methodName="test_exact_qualified_graph_passes")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        root_manifest = fixture.root / "Cargo.toml"
        # Construct the reviewed cohort, independently of the current product manifest.
        root_manifest.write_text('[workspace]\n[patch.crates-io]\n'
            + f'candle-kernels = {{ git = "https://github.com/coreyt/candle-fathomdb.git", rev = "{K9}" }}\n'
            + portable_patches())
        driver = fixture.root / "experiments/sdk_drivers/rust/Cargo.toml"
        driver.parent.mkdir(parents=True)
        driver.write_text('[package]\nname="driver"\nversion="0.1.0"\n[workspace]\n'
            + root_manifest.read_text().split('[workspace]\n', 1)[1])
        write_mirrors(fixture.root, MIRRORS)
        result = fixture.check()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_real_locked_git_mirrors_preserve_graph_features_and_kernel_bytes(self):
        def run(args, cwd, home=None):
            result = subprocess.run(args, cwd=cwd, env={**os.environ, "CARGO_HOME": str(home or self.root / "unused"),
                "CARGO_TARGET_DIR": str(self.root / "must-not-build"), "GIT_CONFIG_GLOBAL": os.devnull},
                capture_output=True, text=True, timeout=45, check=False)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            return result.stdout

        def crate(path, name, version, extra=""):
            (path / "src").mkdir(parents=True)
            (path / "src/lib.rs").write_text("")
            (path / "Cargo.toml").write_text(f'[package]\nname="{name}"\nversion="{version}"\nedition="2021"\n' + extra)

        def commit(repo, message):
            run(["git", "add", "."], repo)
            run(["git", "-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid",
                 "-c", "commit.gpgsign=false", "commit", "-qm", message], repo)
            return run(["git", "rev-parse", "HEAD"], repo).strip()

        candle = self.root / "logical-candle"
        candle.mkdir()
        run(["git", "init", "-q"], candle)
        (candle / "Cargo.toml").write_text('[workspace]\nmembers=["core","nn","transformers","kernels"]\nresolver="2"\n')
        crate(candle / "kernels", "candle-kernels", "0.10.2", '[features]\ndefault=[]\nstatic-cudart=[]\n')
        for name in ("core", "nn", "transformers"):
            crate(candle / name, f"candle-{name}-fathomdb", "0.11.0",
                  '[features]\ndefault=[]\ncuda=["candle-kernels/static-cudart"]\n'
                  '[dependencies]\ncandle-kernels={path="../kernels",version="=0.10.2"}\n')
        k9 = commit(candle, "kernel boundary")
        for name in ("core", "nn", "transformers"):
            manifest = candle / name / "Cargo.toml"
            manifest.write_text(manifest.read_text().replace('version="0.11.0"', 'version="0.12.0"'))
        candle_rev = commit(candle, "fork versions")
        cudarc = self.root / "logical-cudarc"
        crate(cudarc, "cudarc-fathomdb", "0.20.0", '[features]\ndefault=[]\nstd=[]\ndriver=[]\n')
        run(["git", "init", "-q"], cudarc)
        cudarc_rev = commit(cudarc, "driver boundary")
        for repo in (candle, cudarc):
            run(["git", "clone", "--bare", str(repo), str(repo) + "-mirror"], self.root)
        candle_url, cudarc_url = candle.as_uri(), cudarc.as_uri()
        candle_mirror, cudarc_mirror = candle_url + "-mirror", cudarc_url + "-mirror"
        consumer = self.root / "consumer"
        deps = ''.join(f'{name}-fathomdb={{version="=0.12.0",features=["cuda"],default-features=false}}\n'
                       for name in ("candle-core", "candle-nn", "candle-transformers"))
        deps += 'cudarc-fathomdb={version="=0.20.0",features=["std","driver"],default-features=false}\n'
        patches = ''.join(f'{name}-fathomdb={{git="{candle_url}",rev="{candle_rev}"}}\n'
                          for name in ("candle-core", "candle-nn", "candle-transformers"))
        patches += f'cudarc-fathomdb={{git="{cudarc_url}",rev="{cudarc_rev}"}}\n'
        kernel = f'candle-kernels={{git="{candle_mirror}",rev="{k9}"}}\n'
        crate(consumer, "consumer", "0.1.0", '[workspace]\n[dependencies]\n' + deps
              + '[patch.crates-io]\n' + patches + kernel + f'[patch."{candle_url}"]\n' + kernel)

        def seed(home, candle_source, cudarc_source):
            seed_dir = self.root / home.name
            crate(seed_dir, "seed", "0.1.0", '[workspace]\n[dependencies]\n'
                  + f'core={{package="candle-core-fathomdb",git="{candle_source}",rev="{candle_rev}"}}\n'
                  + f'kernel={{package="candle-kernels",git="{candle_mirror}",rev="{k9}"}}\n'
                  + f'cudarc={{package="cudarc-fathomdb",git="{cudarc_source}",rev="{cudarc_rev}"}}\n')
            run(["cargo", "metadata", "--format-version=1"], seed_dir, home)

        baseline_home = self.root / "home-baseline"
        seed(baseline_home, candle_url, cudarc_url)
        command = ["cargo", "metadata", "--offline", "--format-version=1"]
        before = json.loads(run(command, consumer, baseline_home))
        lock = (consumer / "Cargo.lock").read_bytes()
        before_tree = run(["git", "rev-parse", f"{k9}:kernels"], candle).strip()
        self.assertEqual(before_tree, run(["git", "rev-parse", f"{candle_rev}:kernels"], candle).strip())
        candle.rename(self.root / "disconnected-candle")
        cudarc.rename(self.root / "disconnected-cudarc")
        mirror_home = self.root / "home-mirror"
        seed(mirror_home, candle_mirror, cudarc_mirror)
        mirrors = {name: dict(spec) for name, spec in MIRRORS.items()}
        for name, spec in mirrors.items():
            is_candle = "candle" in name
            spec["git"] = (candle_mirror if is_candle else cudarc_mirror) if name.endswith("https") else (candle_url if is_candle else cudarc_url)
            spec["rev"] = candle_rev if is_candle else cudarc_rev
        write_mirrors(consumer, mirrors)
        after = json.loads(run([*command, "--locked"], consumer, mirror_home))
        self.assertFalse(candle.exists())
        self.assertFalse(cudarc.exists())
        self.assertEqual((consumer / "Cargo.lock").read_bytes(), lock)
        def identities(data):
            return sorted((p["name"], p["version"], p["source"]) for p in data["packages"])

        def features(data):
            return sorted((node["id"], node["features"]) for node in data["resolve"]["nodes"])
        self.assertEqual(identities(before), identities(after))
        self.assertEqual(features(before), features(after))
        kernels = [p for p in after["packages"] if p["name"] == "candle-kernels"]
        self.assertEqual(len(kernels), 1)
        self.assertEqual(kernels[0]["source"], f"git+{candle_mirror}?rev={k9}#{k9}")
        self.assertEqual(Path(kernels[0]["manifest_path"]).read_bytes(),
                         (self.root / "disconnected-candle/kernels/Cargo.toml").read_bytes())
        self.assertFalse((self.root / "must-not-build").exists())

    def test_mirror_metadata_is_checker_owned(self):
        data = json.loads((ROOT / "scripts/pinned-override-rot.json").read_text())
        data["cargo_fork_pins"]["source_mirrors"] = {"candle-https": {"git": "https://example.invalid/other"}}
        with self.assertRaises(self.checker.Unverified):
            self.checker.validate_fork_pins(data)


if __name__ == "__main__":
    unittest.main()
