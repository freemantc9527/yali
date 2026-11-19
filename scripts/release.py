#!/usr/bin/env python3
"""Validate release inputs, archive the final App, and assemble both architectures."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import plistlib
import re
import subprocess
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}
VERSION = re.compile(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-(?:alpha|beta|rc)\.[1-9]\d*)?")


def run(*args, cwd=ROOT):
    return subprocess.check_output([str(arg) for arg in args], cwd=cwd, text=True).strip()


def product_version(root=ROOT):
    workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    if not VERSION.fullmatch(workspace):
        raise ValueError(f"Unsupported release version: {workspace}")
    versions = [workspace]
    for name in ("src-tauri/tauri.conf.json", "ui/package.json"):
        versions.append(json.loads((root / name).read_text())["version"])
    packages = {p["name"]: p["version"] for p in tomllib.loads((root / "Cargo.lock").read_text())["package"]}
    for name, manifest in (("yali-core", "crates/yali-core/Cargo.toml"),
                           ("yali-cli", "crates/yali-cli/Cargo.toml"),
                           ("yali-app", "src-tauri/Cargo.toml")):
        if tomllib.loads((root / manifest).read_text())["package"]["version"] != {"workspace": True}:
            raise ValueError(f"{name} must inherit the workspace version")
        versions.append(packages[name])
    if set(versions) != {workspace}:
        raise ValueError(f"Product versions disagree: {versions}")
    return workspace


def check_tag(version, tag):
    if tag and tag != f"v{version}":
        raise ValueError(f"Tag {tag!r} does not match v{version}")


def digest(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def write_new(path, text):
    with path.open("x", encoding="utf-8") as file:
        file.write(text)


def build_provenance(commit):
    if os.environ.get("GITHUB_ACTIONS") != "true":
        return {"buildOrigin": "local-validation"}
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    if (not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository)
            or not re.fullmatch(r"[1-9]\d*", run_id)
            or os.environ.get("GITHUB_SHA") != commit):
        raise ValueError("GitHub build context does not match the candidate")
    return {"buildOrigin": "github-actions", "repository": repository, "runId": run_id,
            "runUrl": f"https://github.com/{repository}/actions/runs/{run_id}"}


def inspect_archive(path):
    with zipfile.ZipFile(path) as archive:
        if archive.testzip() is not None:
            raise ValueError("Archive CRC check failed")
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise ValueError("Duplicate archive members")
        for name in names:
            parts = PurePosixPath(name).parts
            if not parts or name.startswith("/") or ".." in parts or parts[0] not in {"Yali.app", "__MACOSX"}:
                raise ValueError(f"Unsafe archive member: {name}")
        for name in ("yali-app", "yali"):
            if f"Yali.app/Contents/MacOS/{name}" not in names:
                raise ValueError(f"Archive missing {name}")
        plist = plistlib.loads(archive.read("Yali.app/Contents/Info.plist"))
        runtime = json.loads(archive.read("Yali.app/Contents/Resources/yali-runtime.json"))
    return plist, runtime


def package(app, output, arch):
    version = product_version()
    commit = run("git", "rev-parse", "HEAD")
    app = app.resolve()
    if app.name != "Yali.app":
        raise ValueError("Expected the final Yali.app")
    runtime = json.loads((app / "Contents/Resources/yali-runtime.json").read_text())
    if runtime["target"] != TARGETS[arch]:
        raise ValueError(f"Runtime target does not match {arch}")
    for binary in ("yali-app", "yali"):
        if run("/usr/bin/lipo", "-archs", app / "Contents/MacOS" / binary) != arch:
            raise ValueError(f"Wrong architecture: {binary}")
    if run(app / "Contents/MacOS/yali", "--version") != f"yali {version}":
        raise ValueError("Bundled CLI version differs from source")
    run("bash", ROOT / "scripts/bundle-check.sh", app)
    output.mkdir(parents=True, exist_ok=True)
    stem = f"Yali_{version}_macos_{arch}"
    archive = output / f"{stem}.zip"
    manifest = output / f"{stem}.json"
    if any(path.exists() or path.is_symlink() for path in (archive, manifest)):
        raise ValueError("Release output already exists; use a fresh directory")
    run("/usr/bin/ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", app, archive.resolve())
    plist, archived_runtime = inspect_archive(archive)
    if archived_runtime != runtime or plist["CFBundleIdentifier"] != "app.yali":
        raise ValueError("Archive metadata differs from the final App")
    with tempfile.TemporaryDirectory(prefix="yali-release-roundtrip-") as temporary:
        run("/usr/bin/ditto", "-x", "-k", archive.resolve(), temporary)
        run("bash", ROOT / "scripts/bundle-check.sh", Path(temporary) / "Yali.app")
    metadata = {
        "version": version, "commit": commit, "arch": arch,
        **build_provenance(commit),
        "sourceDirty": bool(run("git", "status", "--porcelain", "--untracked-files=normal")),
        "archive": archive.name, "sha256": digest(archive), "runtime": runtime,
        "bundleVersion": plist["CFBundleVersion"],
        "bundleShortVersion": plist["CFBundleShortVersionString"],
        "tools": {tool: run(*command) for tool, command in {
            "rust": ("rustc", "--version"), "node": ("node", "--version"),
            "pnpm": ("pnpm", "--version"), "tauri": ("cargo", "tauri", "--version"),
            "macOS": ("/usr/bin/sw_vers", "-productVersion"),
        }.items()},
    }
    write_new(manifest, json.dumps(metadata, ensure_ascii=False, indent=2) + "\n")
    print(f"Packaged {archive.name}: {metadata['sha256']}")


def assemble(directory, version, commit):
    if not VERSION.fullmatch(version) or not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("Invalid version or commit")
    provenance = build_provenance(commit)
    if provenance["buildOrigin"] != "github-actions":
        raise ValueError("Release assembly requires a GitHub Actions run; local packages are validation-only")
    expected = {f"Yali_{version}_macos_{arch}.{suffix}" for arch in TARGETS for suffix in ("zip", "json")}
    if {path.name for path in directory.iterdir()} != expected:
        raise ValueError("Expected exactly two architecture archives and their manifests")
    checksums, rows = [], []
    for arch, target in TARGETS.items():
        stem = f"Yali_{version}_macos_{arch}"
        manifest = directory / f"{stem}.json"
        archive = directory / f"{stem}.zip"
        if any(path.is_symlink() or not path.is_file() for path in (manifest, archive)):
            raise ValueError("Release inputs must be regular files")
        data = json.loads(manifest.read_text())
        if (data["version"], data["commit"], data["arch"], data["archive"]) != (version, commit, arch, archive.name):
            raise ValueError(f"Manifest does not match candidate: {manifest.name}")
        if data["sourceDirty"] is not False:
            raise ValueError(f"Candidate has uncommitted changes: {manifest.name}")
        if any(data.get(key) != value for key, value in provenance.items()):
            raise ValueError(f"Archive must come from this GitHub Actions run: {manifest.name}")
        if data["sha256"] != digest(archive):
            raise ValueError(f"Archive checksum mismatch: {archive.name}")
        plist, runtime = inspect_archive(archive)
        minimum = runtime["minimumSystemVersion"]
        if (runtime != data["runtime"] or runtime["target"] != target
                or not re.fullmatch(r"\d+\.\d+(?:\.\d+)?", minimum)
                or plist["CFBundleIdentifier"] != "app.yali"
                or plist["CFBundleShortVersionString"] != data["bundleShortVersion"]
                or plist["CFBundleVersion"] != data["bundleVersion"]
                or plist["LSMinimumSystemVersion"] != minimum):
            raise ValueError(f"Archive metadata mismatch: {archive.name}")
        rows.append(f"| {arch} | {minimum} | `{archive.name}` |")
        for path in (archive, manifest):
            checksums.append(f"{digest(path)}  {path.name}\n")
    notes = f"""Yali {version}

源码提交：`{commit}`

| 架构 | 最低 macOS | 下载文件 |
|---|---|---|
{chr(10).join(rows)}

下载对应架构的 zip，解压后将 Yali.app 放入 /Applications 或 ~/Applications。
设置中的自动化页面可安装 Finder 操作。CLI 与图片引擎已内嵌，无需 Homebrew。

本版本采用 ad-hoc 签名，未进行 Developer ID 签名或 Apple 公证。
首次打开按 macOS 的隐私与安全性提示处理；不要关闭 Gatekeeper。

构建与自动检查通过不代表人工验收完成。公开发布前请核对本提交 docs/STATE.md
与任务卡中的人工待验和已知限制，并补充本版用户可见变化。
本流水线不包含应用内自动更新。依赖与工具版本见各架构 JSON；校验和见 SHA256SUMS。
"""
    write_new(directory / "SHA256SUMS", "".join(checksums))
    write_new(directory / "RELEASE_NOTES.md", notes)
    print("Both architectures validated; release notes and SHA256SUMS generated")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("check")
    check.add_argument("--tag", default="")
    check.add_argument("--output", type=Path)
    pack = commands.add_parser("package")
    pack.add_argument("--app", type=Path, default=ROOT / "target/release/bundle/macos/Yali.app")
    pack.add_argument("--output", type=Path, required=True)
    pack.add_argument("--arch", choices=TARGETS, required=True)
    join = commands.add_parser("assemble")
    join.add_argument("--directory", type=Path, required=True)
    join.add_argument("--version", required=True)
    join.add_argument("--commit", required=True)
    args = parser.parse_args()
    if args.command == "check":
        version = product_version()
        check_tag(version, args.tag)
        commit = run("git", "rev-parse", "HEAD")
        if args.tag:
            run("git", "merge-base", "--is-ancestor", commit, "origin/main")
            if run("git", "rev-parse", f"refs/tags/{args.tag}^{{commit}}") != commit:
                raise ValueError("Tag does not point at the checked-out commit")
        values = f"version={version}\ncommit={commit}\nprerelease={str('-' in version).lower()}\n"
        if args.output:
            with args.output.open("a", encoding="utf-8") as output:
                output.write(values)
        print(values, end="")
    elif args.command == "package":
        package(args.app, args.output, args.arch)
    else:
        assemble(args.directory, args.version, args.commit)


if __name__ == "__main__":
    main()
