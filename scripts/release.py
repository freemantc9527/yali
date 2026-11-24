#!/usr/bin/env python3
"""Validate and assemble macOS and Windows release archives from one candidate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import tempfile
import tomllib

import windows_release
from contextlib import contextmanager

ROOT = Path(__file__).resolve().parent.parent
TARGETS = {"arm64": "aarch64-apple-darwin", "x86_64": "x86_64-apple-darwin"}
VERSION = re.compile(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)-alpha")


def run(*args, cwd=ROOT):
    return subprocess.check_output([str(arg) for arg in args], cwd=cwd, text=True).strip()


def product_version(root=ROOT):
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    if not VERSION.fullmatch(workspace):
        raise ValueError(f"Unsupported release version: {workspace}")
    versions = [workspace]
    for name in ("src-tauri/tauri.conf.json", "ui/package.json"):
        versions.append(json.loads((root / name).read_text(encoding="utf-8"))["version"])
    packages = {p["name"]: p["version"] for p in tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))["package"]}
    for name, manifest in (("yali-core", "crates/yali-core/Cargo.toml"),
                           ("yali-cli", "crates/yali-cli/Cargo.toml"),
                           ("yali-app", "src-tauri/Cargo.toml")):
        if tomllib.loads((root / manifest).read_text(encoding="utf-8"))["package"]["version"] != {"workspace": True}:
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


@contextmanager
def mounted_image(path):
    with tempfile.TemporaryDirectory(prefix="yali-dmg-mount-") as temporary:
        mount = Path(temporary) / "volume"
        mount.mkdir()
        run("/usr/bin/hdiutil", "verify", path.resolve())
        run("/usr/bin/hdiutil", "attach", path.resolve(), "-readonly", "-nobrowse", "-mountpoint", mount)
        try:
            yield mount
        finally:
            run("/usr/bin/hdiutil", "detach", mount)


def inspect_archive(path):
    with mounted_image(path) as mount:
        app = mount / "Yali.app"
        if app.is_symlink() or not app.is_dir():
            raise ValueError("Disk image missing Yali.app")
        for name in ("yali-app", "yali"):
            if not (app / "Contents/MacOS" / name).is_file():
                raise ValueError(f"Disk image missing {name}")
        plist = plistlib.loads((app / "Contents/Info.plist").read_bytes())
        runtime = json.loads((app / "Contents/Resources/yali-runtime.json").read_text(encoding="utf-8"))
    return plist, runtime


def package(app, output, arch):
    version = product_version()
    commit = run("git", "rev-parse", "HEAD")
    app = app.resolve()
    if app.name != "Yali.app":
        raise ValueError("Expected the final Yali.app")
    runtime = json.loads((app / "Contents/Resources/yali-runtime.json").read_text(encoding="utf-8"))
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
    archive = output / f"{stem}.dmg"
    manifest = output / f"{stem}.json"
    if any(path.exists() or path.is_symlink() for path in (archive, manifest)):
        raise ValueError("Release output already exists; use a fresh directory")
    with tempfile.TemporaryDirectory(prefix="yali-release-stage-") as temporary:
        stage = Path(temporary)
        run("/usr/bin/ditto", app, stage / "Yali.app")
        (stage / "Applications").symlink_to("/Applications")
        run("/usr/bin/hdiutil", "create", "-fs", "HFS+", "-format", "UDZO", "-volname", "Yali",
            "-srcfolder", stage, archive.resolve())
    plist, archived_runtime = inspect_archive(archive)
    if archived_runtime != runtime or plist["CFBundleIdentifier"] != "app.yali":
        raise ValueError("Disk image metadata differs from the final App")
    with mounted_image(archive) as mount:
        run("bash", ROOT / "scripts/bundle-check.sh", mount / "Yali.app")
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
    expected = {f"Yali_{version}_macos_{arch}.{suffix}" for arch in TARGETS for suffix in ("dmg", "json")}
    expected.update(windows_release.names(version).values())
    if {path.name for path in directory.iterdir()} != expected:
        raise ValueError("Expected exactly two macOS architecture archives/manifests and the complete Windows release set")
    checksums, rows = [], []
    for arch, target in TARGETS.items():
        stem = f"Yali_{version}_macos_{arch}"
        manifest = directory / f"{stem}.json"
        archive = directory / f"{stem}.dmg"
        if any(path.is_symlink() or not path.is_file() for path in (manifest, archive)):
            raise ValueError("Release inputs must be regular files")
        data = json.loads(manifest.read_text(encoding="utf-8"))
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
    for path in windows_release.validate(directory, version, commit, provenance):
        checksums.append(f"{digest(path)}  {path.name}\n")
    win = windows_release.names(version)
    notes = f"""Yali {version}

源码提交：`{commit}`

| 架构 | 最低 macOS | 下载文件 |
|---|---|---|
{chr(10).join(rows)}

下载对应架构的 DMG，打开后将 Yali.app 拖入应用程序文件夹。
设置中的自动化页面可安装 Finder 操作。CLI 与图片引擎已内嵌，无需 Homebrew。

App 不再内置 FFmpeg；没有它仍可处理图片和视觉对比，SSIM 分数会隐藏。
在“应用设置 → 通用 → 质量检测”查看外部 FFmpeg 路径与 SSIM 支持，并可重新检查。
设置分为通用、自动化和关于；窄屏导航只显示图标与标题。

Windows 11 x64 实验包：`{win["archive"]}`。解压后运行 Yali-workbench.exe，CLI 使用 yali.exe。
图片引擎已内嵌，支持 JPEG、PNG、GIF、WebP、BMP、TIFF 输入；GUI 需要已安装 Microsoft Edge WebView2。
对应源码：`{win["sources"]}`；Windows 11 GUI、WebView2 缺失与 SmartScreen 仍待人工验收。

本版本采用 ad-hoc 签名，未进行 Developer ID 签名或 Apple 公证。
首次打开按 macOS 的隐私与安全性提示处理；不要关闭 Gatekeeper。

构建与自动检查通过不代表人工验收完成；已知限制和待验项目见源码中的 docs/STATE.md。
本流水线不包含应用内自动更新。依赖与工具版本见各架构 JSON；校验和见 SHA256SUMS。
"""
    write_new(directory / "SHA256SUMS", "".join(checksums))
    write_new(directory / "RELEASE_NOTES.md", notes)
    print("macOS arm64/x86_64 and Windows x86_64 validated; release notes and SHA256SUMS generated")


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
    attest = commands.add_parser("attest-windows")
    attest.add_argument("--directory", type=Path, required=True)
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
    elif args.command == "attest-windows":
        commit = run("git", "rev-parse", "HEAD")
        windows_release.attest(args.directory, product_version(), commit, build_provenance(commit))
    else:
        assemble(args.directory, args.version, args.commit)


if __name__ == "__main__":
    main()
