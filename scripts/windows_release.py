"""Validate Windows binary/source archives and bind final smoke reports to a release candidate."""
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent
_spec = importlib.util.spec_from_file_location("yali_windows_runtime", ROOT / "scripts/windows-runtime.py")
runtime = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(runtime)


def names(version):
    stem = f"Yali_{version}_windows_x86_64"
    return {
        "archive": stem + "-experiment.zip",
        "sources": "Yali_windows_corresponding-sources.zip",
        "manifest": stem + ".json",
        "dependencies": stem + ".dependencies.json",
        "smoke": stem + ".smoke.json",
        "quality": stem + ".quality.json",
    }


def validate_reports(dependencies, smoke, quality):
    if dependencies.get("result") != "passed" or dependencies.get("peFiles", 0) < 12:
        raise ValueError("Windows dependency verification did not pass")
    for report, minimum_checks in [(smoke, 6), (quality, 7)]:
        if report.get("result") != "passed" or len(report.get("automatic", [])) < minimum_checks or not report.get("manual"):
            raise ValueError("Windows final ZIP smoke is missing or failed")
        paths = report.get("enginePaths", {})
        if set(paths) != set(runtime.TOOLS) or any(not path.lower().endswith(name + ".exe") for name, path in paths.items()):
            raise ValueError("Windows smoke did not exercise all packaged tools")
    scores = quality.get("quality", [])
    if {score.get("engine") for score in scores if isinstance(score, dict)} != {"cjpeg", "pngquant", "cwebp"}:
        raise ValueError("Windows encoding quality references are missing")
    for score in scores:
        values = [score.get("referenceSSIM"), score.get("actualSSIM"), score.get("sizeRatio")]
        if any(not isinstance(value, (int, float)) or not math.isfinite(value) for value in values):
            raise ValueError("Invalid Windows quality measurement")
        reference, actual, ratio = values
        if not (0 < reference <= 1 and 0 < actual <= 1 and 0 < ratio <= 1.03 and actual >= reference * .97):
            raise ValueError("Windows quality regression exceeds 3%")


def inspect_package(archive, sources):
    with tempfile.TemporaryDirectory(prefix="yali-windows-release-") as temporary:
        root = Path(temporary) / "unpacked"
        runtime.extract(archive, root)
        dependency_result = runtime.check(root)
        info = json.loads((root / "build-info.json").read_text(encoding="utf-8"))
        manifest = json.loads((root / "yali-runtime.json").read_text(encoding="utf-8"))
        source_info = json.loads((root / "licenses/SOURCES.json").read_text(encoding="utf-8"))
        if source_info.get("sourceBundle") != sources.name or source_info.get("sha256") != runtime.digest(sources):
            raise ValueError("Windows corresponding source bundle checksum mismatch")
    with zipfile.ZipFile(sources) as source_zip:
        if source_zip.testzip() is not None:
            raise ValueError("Windows corresponding source ZIP is damaged")
        lock_bytes = source_zip.read("scripts/windows-engines.lock.json")
        if hashlib.sha256(lock_bytes).hexdigest() != manifest["engineLockSha256"]:
            raise ValueError("Windows binary and corresponding source locks differ")
        lock = json.loads(lock_bytes)
        for item in lock["archives"]:
            if item["kind"] != "binary":
                if hashlib.sha256(source_zip.read(item["archive"])).hexdigest() != item["sha256"]:
                    raise ValueError("Windows corresponding upstream source checksum mismatch")
        for prefix in ["cargo-vendor/", "cargo-vendor-librsvg/"]:
            if not any(name.startswith(prefix) and name.endswith("Cargo.toml") for name in source_zip.namelist()):
                raise ValueError("Windows corresponding Cargo sources are missing")
    return info, dependency_result


def check_info(info, version, commit, provenance):
    if (info.get("version"), info.get("commit"), info.get("target"), info.get("sourceDirty"),
            info.get("repository"), info.get("runUrl"), info.get("imageEnginesBundled")) != (
            version, commit, runtime.TARGET, False, provenance["repository"], provenance["runUrl"], True):
        raise ValueError("Windows ZIP build does not match this clean candidate/run")


def attest(directory, version, commit, provenance):
    if provenance["buildOrigin"] != "github-actions":
        raise ValueError("Windows release attestation requires GitHub Actions")
    files = names(version)
    info, dependencies = inspect_package(directory / files["archive"], directory / files["sources"])
    check_info(info, version, commit, provenance)
    reports = {
        "dependencies": json.loads((directory / "dependency-report.json").read_text(encoding="utf-8")),
        "smoke": json.loads((directory / "smoke-without-ffmpeg.json").read_text(encoding="utf-8")),
        "quality": json.loads((directory / "smoke-report.json").read_text(encoding="utf-8")),
    }
    if reports["dependencies"] != dependencies:
        raise ValueError("Windows dependency report differs from ZIP readback")
    validate_reports(**reports)
    for key, report in reports.items():
        with (directory / files[key]).open("x", encoding="utf-8") as stream:
            stream.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    metadata = {
        "version": version, "commit": commit, "platform": "windows", "arch": "x86_64",
        "target": runtime.TARGET, "sourceDirty": False, **provenance,
        "validation": {"dependencies": "passed", "smokeWithoutFfmpeg": "passed", "quality": "passed",
                       "windows11Gui": "pending"},
        "buildInfo": info,
        "files": {files[key]: runtime.digest(directory / files[key]) for key in files if key != "manifest"},
    }
    with (directory / files["manifest"]).open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n")


def validate(directory, version, commit, provenance):
    files = names(version)
    for name in files.values():
        path = directory / name
        if path.is_symlink() or not path.is_file():
            raise ValueError("Windows release inputs must be regular files")
    metadata = json.loads((directory / files["manifest"]).read_text(encoding="utf-8"))
    if (metadata.get("version"), metadata.get("commit"), metadata.get("platform"), metadata.get("arch"),
            metadata.get("target"), metadata.get("sourceDirty")) != (
            version, commit, "windows", "x86_64", runtime.TARGET, False):
        raise ValueError("Windows manifest does not match candidate")
    if any(metadata.get(key) != value for key, value in provenance.items()):
        raise ValueError("Windows archive must come from this GitHub Actions run")
    expected = {name for key, name in files.items() if key != "manifest"}
    if set(metadata.get("files", {})) != expected:
        raise ValueError("Windows release file inventory differs")
    for name, checksum in metadata["files"].items():
        if runtime.digest(directory / name) != checksum:
            raise ValueError(f"Windows archive/report checksum mismatch: {name}")
    info, dependencies = inspect_package(directory / files["archive"], directory / files["sources"])
    check_info(info, version, commit, provenance)
    if info != metadata.get("buildInfo"):
        raise ValueError("Windows ZIP provenance differs from attestation")
    reports = {key: json.loads((directory / files[key]).read_text(encoding="utf-8"))
               for key in ["dependencies", "smoke", "quality"]}
    if dependencies != reports["dependencies"]:
        raise ValueError("Windows dependency report differs from ZIP")
    validate_reports(**reports)
    return [directory / name for name in files.values()]
