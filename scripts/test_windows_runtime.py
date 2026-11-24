import importlib.util
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location("windows_runtime", Path(__file__).with_name("windows-runtime.py"))
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


def pe(imported=None, delay=False, machine=0x8664):
    data = bytearray(2048)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 60, 128)
    data[128:132] = b"PE\0\0"
    struct.pack_into("<HH", data, 132, machine, 1)
    struct.pack_into("<H", data, 148, 240)
    struct.pack_into("<H", data, 152, 0x20B)
    struct.pack_into("<I", data, 212, 512)  # SizeOfHeaders
    struct.pack_into("<I", data, 260, 16)  # NumberOfRvaAndSizes
    struct.pack_into("<IIII", data, 400, 1536, 0x1000, 1536, 512)
    if imported:
        struct.pack_into("<II", data, 264 + (13 if delay else 1) * 8, 0x1000, 64)
        if delay:
            struct.pack_into("<II", data, 512, 1, 0x1100)
        else:
            struct.pack_into("<I", data, 524, 0x1100)
        value = imported.encode() + b"\0"
        data[768:768 + len(value)] = value
    return bytes(data)


class WindowsRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="yali-windows-test-")
        self.root = Path(self.temp.name)
        self.addCleanup(self.temp.cleanup)

    def fixture(self):
        tools = {}
        for name, (_, raw) in runtime.TOOLS.items():
            path = self.root / raw
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(pe("kernel32.dll"))
            tools[name] = dict(path=raw, version="1.0", source="https://example.invalid/upstream", license="LICENSE")
        for raw in ["Yali-workbench.exe", "yali.exe"]:
            (self.root / raw).write_bytes(pe("kernel32.dll"))
        for raw in ["LICENSE", "README-Windows.txt", "build-info.json"]:
            (self.root / raw).write_text("{}")
        manifest = dict(schemaVersion=1, target=runtime.TARGET, tools=tools,
                        files={p.relative_to(self.root).as_posix(): runtime.digest(p)
                               for p in self.root.rglob("*") if p.is_file()})
        self.save(manifest)
        return manifest

    def save(self, manifest):
        runtime.write_json(self.root / "yali-runtime.json", manifest)

    def test_final_package_and_missing_tool_fail_closed(self):
        manifest = self.fixture()
        self.assertEqual(runtime.check(self.root)["peFiles"], 12)
        runtime.relative_file(self.root, manifest["tools"]["vips"]["path"]).unlink()
        with self.assertRaisesRegex(ValueError, "missing"):
            runtime.check(self.root)

    def test_corrupt_dependency_fails_checksum_even_when_tool_exists(self):
        manifest = self.fixture()
        dll = self.root / "runtime/bin/private.dll"
        dll.write_bytes(pe())
        manifest["files"]["runtime/bin/private.dll"] = runtime.digest(dll)
        self.save(manifest)
        dll.write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            runtime.check(self.root)

    def test_normal_and_delay_import_closure_and_architecture(self):
        for delay in [False, True]:
            image = self.root / "tool.exe"
            image.write_bytes(pe("private.dll", delay=delay))
            self.assertEqual(runtime.pe_imports(image), ["private.dll"])
        image.write_bytes(pe(machine=0x14C))
        with self.assertRaisesRegex(ValueError, "x64"):
            runtime.pe_imports(image)

    def test_missing_non_system_dll_cannot_be_satisfied_by_host(self):
        manifest = self.fixture()
        tool = self.root / manifest["tools"]["oxipng"]["path"]
        tool.write_bytes(pe("vcruntime140.dll", delay=True))
        manifest["files"][tool.relative_to(self.root).as_posix()] = runtime.digest(tool)
        self.save(manifest)
        with self.assertRaisesRegex(ValueError, "vcruntime140.dll"):
            runtime.check(self.root)
        dll = tool.parent / "vcruntime140.dll"
        dll.write_bytes(pe("kernel32.dll"))
        manifest["files"][dll.relative_to(self.root).as_posix()] = runtime.digest(dll)
        self.save(manifest)
        self.assertEqual(runtime.check(self.root)["result"], "passed")

    def test_paths_cannot_escape_or_reinterpret_windows_drive(self):
        (self.root / "safe").write_bytes(b"x")
        for raw in ["../safe", "/safe", "C:/safe", "C:safe", r"..\safe", r"\\host\safe", "a/../safe", "./safe"]:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                runtime.relative_file(self.root, raw)

    def test_zip_readback_and_traversal(self):
        archive = self.root / "bad.zip"
        with zipfile.ZipFile(archive, "w") as z:
            z.writestr("../escape", b"x")
        with self.assertRaises(ValueError):
            runtime.extract(archive, self.root / "unpacked")
        source = self.root / "source"
        source.mkdir()
        (source / "中文 [test].txt").write_text("回读", encoding="utf-8")
        good = self.root / "good.zip"
        runtime.zip_directory(source, good)
        dest = runtime.extract(good, self.root / "中文 空格")
        self.assertEqual((dest / "中文 [test].txt").read_text(encoding="utf-8"), "回读")

    def test_untracked_pe_and_architecture_mismatch_are_rejected(self):
        manifest = self.fixture()
        rogue = self.root / "runtime/bin/rogue.dll"
        rogue.write_bytes(pe())
        with self.assertRaisesRegex(ValueError, "untracked PE"):
            runtime.check(self.root)
        manifest["files"]["runtime/bin/rogue.dll"] = runtime.digest(rogue)
        manifest["target"] = "i686-pc-windows-msvc"
        self.save(manifest)
        with self.assertRaisesRegex(ValueError, "target"):
            runtime.check(self.root)

    def test_package_seals_and_reads_back_actual_zip_and_source_bundle(self):
        build = self.root / "build"
        work = build / "work"
        (work / "bin").mkdir(parents=True)
        paths, archives = {}, []
        downloads = build / "windows-downloads"
        downloads.mkdir()
        for source in sorted({source for source, _ in runtime.TOOLS.values()}):
            path = work / source
            path.mkdir()
            paths[source] = str(path)
            archive = downloads / (source + ".zip")
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("LICENSE.txt", "upstream test notice")
            archives.append(dict(name=source, kind="source", version="1.0", archive=archive.name,
                                 sha256=runtime.digest(archive), url="https://example.invalid/" + source))
        for name, (source, _) in runtime.TOOLS.items():
            path = Path(paths[source]) / ("bin" if source == "vips" else "") / (name + ".exe")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(pe())
            if source not in ("vips", "oxipng"):
                (work / "bin" / (name + ".exe")).write_bytes(pe())
        (work / "bin/vcruntime140.dll").write_bytes(pe())
        (work / "msvc-licenses").mkdir()
        (work / "msvc-licenses/LICENSE.txt").write_text("Visual Studio test notice")
        (work / "build-tools.json").write_text("{}")
        for folder in ["vendor", "vendor-librsvg"]:
            crate = work / folder / "example"
            crate.mkdir(parents=True)
            (crate / "Cargo.toml").write_text("[package]\\nname='example'\\nversion='1.0.0'\\n")
            (crate / "LICENSE").write_text("MIT test notice")
        (build / "release").mkdir()
        for name in ["yali.exe", "yali-app.exe"]:
            (build / "release" / name).write_bytes(pe())
        lock = self.root / "lock.json"
        runtime.write_json(lock, dict(archives=archives))
        runtime.write_json(build / "windows-prepared.json",
                           dict(work=str(work), sources=paths, lockSha256=runtime.digest(lock)))
        output = self.root / "assets"
        with patch.object(runtime, "LOCK", lock), patch.object(runtime.subprocess, "check_output", return_value=""):
            runtime.package(build, output)
        unpacked = output / "解压 验证 [Yali]"
        self.assertEqual(runtime.check(unpacked)["result"], "passed")
        for line in (output / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
            expected, name = line.split("  ")
            self.assertEqual(runtime.digest(output / name), expected)
        self.assertTrue((output / "Yali_windows_corresponding-sources.zip").is_file())

    def test_zip_normalizes_unrepresentable_upstream_timestamp(self):
        source = self.root / "old-source"
        source.mkdir()
        path = source / "COPYING"
        path.write_bytes(b"original upstream notice")
        os.utime(path, (1, 1))
        archive = self.root / "old-source.zip"
        runtime.zip_directory(source, archive)
        with zipfile.ZipFile(archive) as z:
            self.assertEqual(z.read("COPYING"), path.read_bytes())
            self.assertEqual(z.getinfo("COPYING").date_time[:3], (1980, 1, 1))

    def test_upstream_cargo_workspace_is_independent_of_parent_and_submodule(self):
        (self.root / "Cargo.toml").write_text('[workspace]\nmembers=["app"]\n', encoding="utf-8")
        for folder, name in [("app", "app"), ("upstream", "pngquant"),
                             ("upstream/lib", "imagequant"), ("upstream/lib/imagequant-sys", "imagequant-sys")]:
            path = self.root / folder
            (path / "src").mkdir(parents=True)
            text = f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2021"\n'
            if name == "pngquant":
                text += '[dependencies]\nimagequant-sys={path="lib/imagequant-sys"}\n'
            elif name == "imagequant":
                text += '[workspace]\nmembers=["imagequant-sys"]\n'
            elif name == "imagequant-sys":
                text += '[dependencies]\nimagequant={path=".."}\n'
            (path / "Cargo.toml").write_text(text, encoding="utf-8")
            (path / "src/lib.rs").write_text("", encoding="utf-8")
        source = self.root / "upstream"
        args = ["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1",
                "--manifest-path", str(source / "Cargo.toml")]
        before = subprocess.run(args, capture_output=True, text=True)
        self.assertNotEqual(before.returncode, 0)
        self.assertIn("workspace", before.stderr)
        runtime.isolate_pngquant(source)
        after = subprocess.run(args, capture_output=True, text=True)
        self.assertEqual(after.returncode, 0, after.stderr)
        self.assertEqual(Path(json.loads(after.stdout)["workspace_root"]), source)

    def test_lock_has_fixed_digests_and_cargo_lock(self):
        lock = json.loads(runtime.LOCK.read_text(encoding="utf-8"))
        self.assertEqual(lock["target"], runtime.TARGET)
        names = set()
        self.assertFalse(lock["qualityTool"]["distributed"])
        self.assertNotIn(lock["qualityTool"]["name"], {item["name"] for item in lock["archives"]})
        for item in [*lock["archives"], lock["qualityTool"]]:
            self.assertRegex(item["sha256"], "^[0-9a-f]{64}$")
            self.assertTrue(item["url"].startswith("https://"))
            self.assertNotIn(item["name"], names)
            names.add(item["name"])
        self.assertEqual(lock["pngquantCargoLockSha256"],
                         runtime.digest(runtime.ROOT / "scripts/windows-pngquant.Cargo.lock"))


if __name__ == "__main__":
    unittest.main()
