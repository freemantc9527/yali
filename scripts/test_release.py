import json
import os
from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch

import release


class ReleaseTests(unittest.TestCase):
    version = "0.1.0-alpha"
    commit = "a" * 40

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="yali-release-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.github_environment = {
            "GITHUB_ACTIONS": "true", "GITHUB_REPOSITORY": "example/yali",
            "GITHUB_RUN_ID": "123456", "GITHUB_SHA": self.commit,
        }
        environment = patch.dict(os.environ, self.github_environment)
        environment.start()
        inspector = patch.object(release, "inspect_archive", side_effect=self.inspect_image)
        inspector.start()
        self.addCleanup(inspector.stop)
        self.addCleanup(environment.stop)

    def inspect_image(self, path):
        arch = "x86_64" if "x86_64" in path.name else "arm64"
        runtime = {"target": release.TARGETS[arch], "minimumSystemVersion": "15.0"}
        plist = {"CFBundleIdentifier": "app.yali", "CFBundleVersion": self.version,
                 "CFBundleShortVersionString": self.version, "LSMinimumSystemVersion": "15.0"}
        return plist, runtime

    def source(self):
        (self.root / "Cargo.toml").write_text(f'[workspace.package]\nversion = "{self.version}"\n')
        packages = []
        for name, folder in (("yali-core", "crates/yali-core"), ("yali-cli", "crates/yali-cli"), ("yali-app", "src-tauri")):
            path = self.root / folder
            path.mkdir(parents=True, exist_ok=True)
            (path / "Cargo.toml").write_text('[package]\nversion.workspace = true\n')
            packages.append(f'[[package]]\nname = "{name}"\nversion = "{self.version}"\n')
        (self.root / "Cargo.lock").write_text("\n".join(packages))
        for name in ("ui/package.json", "src-tauri/tauri.conf.json"):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({"version": self.version}))

    def artifact(self, arch):
        stem = f"Yali_{self.version}_macos_{arch}"
        archive = self.root / f"{stem}.dmg"
        runtime = {"target": release.TARGETS[arch], "minimumSystemVersion": "15.0"}
        plist = {"CFBundleIdentifier": "app.yali", "CFBundleVersion": self.version,
                 "CFBundleShortVersionString": self.version, "LSMinimumSystemVersion": "15.0"}
        archive.write_bytes(f"test image {arch}".encode())
        metadata = {"version": self.version, "commit": self.commit, "arch": arch, "sourceDirty": False,
                    **release.build_provenance(self.commit),
                    "archive": archive.name, "sha256": release.digest(archive), "runtime": runtime,
                    "bundleVersion": self.version, "bundleShortVersion": self.version}
        manifest = self.root / f"{stem}.json"
        manifest.write_text(json.dumps(metadata))
        return archive, manifest

    def pair(self):
        return [self.artifact(arch) for arch in release.TARGETS]

    def assemble(self):
        release.assemble(self.root, self.version, self.commit)

    def test_inherited_and_locked_versions_must_agree(self):
        self.source()
        self.assertEqual(release.product_version(self.root), self.version)
        (self.root / "ui/package.json").write_text('{"version":"0.0.1"}')
        with self.assertRaisesRegex(ValueError, "disagree"):
            release.product_version(self.root)

    def test_crates_cannot_override_shared_version(self):
        self.source()
        (self.root / "src-tauri/Cargo.toml").write_text('[package]\nversion = "0.0.1"\n')
        with self.assertRaisesRegex(ValueError, "inherit"):
            release.product_version(self.root)

    def test_release_tag_must_match_version_exactly(self):
        release.check_tag(self.version, "v" + self.version)
        release.check_tag(self.version, "")
        for tag in ("v0.1.0", "v0.1.0-alpha.1", "v0.1.0-beta", "v0.1.0-alpha\nattack", "../main"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.check_tag(self.version, tag)

    def test_both_architectures_generate_four_checksums(self):
        self.pair()
        self.assemble()
        lines = (self.root / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(lines), 4)
        for line in lines:
            checksum, filename = line.split("  ")
            self.assertEqual(release.digest(self.root / filename), checksum)
        notes = (self.root / "RELEASE_NOTES.md").read_text(encoding="utf-8")
        self.assertIn("| arm64 | 15.0 |", notes)
        self.assertIn("| x86_64 | 15.0 |", notes)
        self.assertIn("App 不再内置 FFmpeg", notes)
        self.assertIn("SSIM 分数会隐藏", notes)
        self.assertIn("Windows 尚未实机验证", notes)
        self.assertIn("当前仅提供 macOS 资产", notes)

    def test_missing_architecture_blocks_release(self):
        self.artifact("arm64")
        with self.assertRaisesRegex(ValueError, "exactly two"):
            self.assemble()
        self.assertFalse((self.root / "SHA256SUMS").exists())

    def test_mixed_commits_and_runtime_targets_block_release(self):
        pairs = self.pair()
        manifest = pairs[1][1]
        original = json.loads(manifest.read_text(encoding="utf-8"))
        for key, value in (("commit", "b" * 40), ("sourceDirty", True),
                           ("runtime", {"target": release.TARGETS["arm64"], "minimumSystemVersion": "15.0"}),
                           ("archive", "../outside.dmg")):
            data = {**original, key: value}
            manifest.write_text(json.dumps(data))
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.assemble()
            self.assertFalse((self.root / "SHA256SUMS").exists())

    def test_changed_archive_blocks_release(self):
        pairs = self.pair()
        with pairs[0][0].open("ab") as file:
            file.write(b"tampered")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            self.assemble()

    def test_local_packages_are_rejected_even_with_clean_sources(self):
        pairs = self.pair()
        manifest = pairs[0][1]
        data = json.loads(manifest.read_text(encoding="utf-8"))
        data["buildOrigin"] = "local-validation"
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "this GitHub Actions run"):
            self.assemble()

    def test_artifacts_from_another_run_or_repository_are_rejected(self):
        pairs = self.pair()
        manifest = pairs[1][1]
        original = json.loads(manifest.read_text(encoding="utf-8"))
        for key, value in (("runId", "987654"), ("repository", "another/project")):
            manifest.write_text(json.dumps({**original, key: value}))
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "this GitHub Actions run"):
                self.assemble()

    def test_local_assembly_is_not_a_release_path(self):
        self.pair()
        with patch.dict(os.environ, {"GITHUB_ACTIONS": "false"}):
            self.assertEqual(release.build_provenance(self.commit), {"buildOrigin": "local-validation"})
            with self.assertRaisesRegex(ValueError, "validation-only"):
                self.assemble()

    def test_github_context_must_match_source_commit(self):
        with patch.dict(os.environ, {"GITHUB_SHA": "b" * 40}):
            with self.assertRaisesRegex(ValueError, "context does not match"):
                release.build_provenance(self.commit)

    def test_invalid_disk_image_blocks_release_even_with_matching_checksum(self):
        self.pair()
        with patch.object(release, "inspect_archive", side_effect=ValueError("Invalid disk image")):
            with self.assertRaisesRegex(ValueError, "Invalid disk image"):
                self.assemble()
        self.assertFalse((self.root / "SHA256SUMS").exists())

    def test_symlinked_manifest_is_rejected(self):
        pairs = self.pair()
        manifest = pairs[0][1]
        outside = self.root.parent / f"{self.root.name}-manifest.json"
        manifest.rename(outside)
        self.addCleanup(outside.unlink)
        manifest.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "regular files"):
            self.assemble()

    def test_outputs_are_never_overwritten(self):
        self.pair()
        path = self.root / "SHA256SUMS"
        path.write_text("existing release")
        with self.assertRaises(ValueError):
            self.assemble()
        self.assertEqual(path.read_text(encoding="utf-8"), "existing release")


if __name__ == "__main__":
    unittest.main()
