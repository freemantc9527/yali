#!/usr/bin/env python3
"""Prepare pinned Windows engines, package a portable ZIP, and verify its PE closure."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import shutil
import struct
import subprocess
import tarfile
import tempfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
LOCK = ROOT / "scripts/windows-engines.lock.json"
TARGET = "x86_64-pc-windows-msvc"
TOOLS = {
    "vips": ("vips", "runtime/vips/bin/vips.exe"),
    "vipsheader": ("vips", "runtime/vips/bin/vipsheader.exe"),
    "vipsthumbnail": ("vips", "runtime/vips/bin/vipsthumbnail.exe"),
    "cjpeg": ("mozjpeg", "runtime/bin/cjpeg.exe"),
    "jpegtran": ("mozjpeg", "runtime/bin/jpegtran.exe"),
    "pngquant": ("pngquant", "runtime/bin/pngquant.exe"),
    "oxipng": ("oxipng", "runtime/bin/oxipng.exe"),
    "gifsicle": ("gifsicle", "runtime/bin/gifsicle.exe"),
    "cwebp": ("libwebp", "runtime/bin/cwebp.exe"),
    "gif2webp": ("libwebp", "runtime/bin/gif2webp.exe"),
}
# OS API contracts and inbox DLLs only. VC++/MinGW runtimes must resolve inside ZIP.
SYSTEM_DLLS = set("""
advapi32 avrt bcrypt bcryptprimitives cabinet cfgmgr32 clbcatq comctl32 comdlg32
crypt32 cryptbase cryptnet cryptsp d2d1 d3d11 d3d12 dcomp dinput8 dnsapi dwmapi
dwrite dxgi dxguid gdi32 gdiplus glu32 imm32 iphlpapi kernel32 kernelbase
mf mfplat mfreadwrite mfuuid mmdevapi mpr msvcrt mswsock msimg32 ncrypt netapi32
normaliz ntdll ole32 oleacc oleaut32 opengl32 powrprof propsys psapi rasapi32
rpcrt4 runtimeobject secur32 setupapi shcore shell32 shlwapi sspicli ucrtbase
urlmon user32 userenv usp10 uxtheme version wevtapi windowscodecs winhttp
wininet winmm winspool wintrust wldap32 ws2_32 wtsapi32 winnsi hid winusb
""".split())


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def relative_file(root, raw):
    posix, windows = PurePosixPath(raw), PureWindowsPath(raw)
    if (not raw or "\\" in raw or ":" in raw or posix.is_absolute()
            or windows.is_absolute() or any(p in ("", ".", "..") for p in raw.split("/"))):
        raise ValueError(f"unsafe package path: {raw}")
    path = root / posix
    if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()) or not path.is_file():
        raise ValueError(f"missing or escaping package file: {raw}")
    return path


def pe_imports(path):
    """Read normal and delay imports without a Windows toolchain or third-party parser."""
    data = path.read_bytes()

    def unpack(fmt, offset):
        size = struct.calcsize(fmt)
        if offset < 0 or offset + size > len(data):
            raise ValueError(f"truncated PE: {path}")
        return struct.unpack_from(fmt, data, offset)

    if data[:2] != b"MZ":
        raise ValueError(f"not PE: {path}")
    pe, = unpack("<I", 60)
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError(f"invalid PE signature: {path}")
    machine, sections = unpack("<HH", pe + 4)
    optional_size, = unpack("<H", pe + 20)
    optional = pe + 24
    magic, = unpack("<H", optional)
    if machine != 0x8664 or magic != 0x20B or optional_size < 128:
        raise ValueError(f"requires x64 PE32+: {path}")
    image_base, = unpack("<Q", optional + 24)
    header_size, = unpack("<I", optional + 60)
    table = optional + optional_size

    def rva(value):
        if value < header_size:
            return value
        for i in range(sections):
            virtual_size, address, raw_size, raw_offset = unpack("<IIII", table + i * 40 + 8)
            if address <= value < address + max(virtual_size, raw_size):
                return raw_offset + value - address
        raise ValueError(f"unmapped PE RVA {value:x}: {path}")

    def name(value):
        offset = rva(value)
        end = data.find(b"\0", offset)
        if end < offset or end - offset > 260:
            raise ValueError(f"invalid DLL name: {path}")
        text = data[offset:end].decode("ascii").lower()
        if not text.endswith(".dll") or any(c in text for c in "/\\:"):
            raise ValueError(f"invalid DLL import {text}: {path}")
        return text

    count, = unpack("<I", optional + 108)
    imports = set()
    for index, width, name_offset in [(1, 20, 12), (13, 32, 4)]:
        if index >= count:
            continue
        address, size = unpack("<II", optional + 112 + index * 8)
        if not address:
            continue
        offset = rva(address)
        for _ in range(min(size // width + 1, 4096)):
            fields = unpack("<" + "I" * (width // 4), offset)
            if not any(fields):
                break
            value = fields[name_offset // 4]
            if index == 13 and not fields[0] & 1:
                value -= image_base
            imports.add(name(value))
            offset += width
        else:
            raise ValueError(f"unterminated import table: {path}")
    return sorted(imports)


def system_dll(name):
    return (name.startswith(("api-ms-win-", "ext-ms-win-"))
            or name.removesuffix(".dll") in SYSTEM_DLLS)


def check(directory):
    directory = directory.resolve()
    manifest = json.loads((directory / "yali-runtime.json").read_text(encoding="utf-8"))
    if manifest["schemaVersion"] != 1 or manifest["target"] != TARGET:
        raise ValueError("unsupported runtime schema/target")
    if set(manifest["tools"]) != set(TOOLS):
        raise ValueError("incomplete tool map")
    tracked = manifest["files"]
    for name, tool in manifest["tools"].items():
        if tool["path"] not in tracked or not tool["path"].endswith(".exe"):
            raise ValueError(f"untracked tool: {name}")
        relative_file(directory, tool["license"])
        if not tool["version"] or not tool["source"]:
            raise ValueError(f"missing tool provenance: {name}")
    for raw, expected in tracked.items():
        if digest(relative_file(directory, raw)) != expected:
            raise ValueError(f"SHA-256 mismatch: {raw}")
    for required in ["Yali-workbench.exe", "yali.exe", "README-Windows.txt", "LICENSE", "build-info.json"]:
        relative_file(directory, required)
    binaries = sorted(p for p in directory.rglob("*") if p.suffix.lower() in (".exe", ".dll"))
    for image in binaries:
        if image.relative_to(directory).as_posix() not in tracked:
            raise ValueError(f"untracked PE: {image}")
        # Match runtime::command_environment: per-tool directory, shared bin, system.
        dirs = [image.parent, directory / "runtime/bin"]
        if image.is_relative_to(directory / "runtime/vips"):
            dirs.insert(1, directory / "runtime/vips/bin")
        found = {}
        for folder in dirs:
            if folder.is_dir():
                for path in folder.iterdir():
                    if path.is_file():
                        found.setdefault(path.name.lower(), path)
        for imported in pe_imports(image):
            if not system_dll(imported) and imported not in found:
                raise ValueError(f"missing DLL {imported} required by {image.relative_to(directory)}")
    return {"target": TARGET, "peFiles": len(binaries), "trackedFiles": len(tracked), "result": "passed"}


def download(item, downloads):
    path = downloads / item["archive"]
    if not path.exists():
        staged = path.with_suffix(path.suffix + ".partial")
        request = urllib.request.Request(item["url"], headers={"User-Agent": "Yali-Windows-runtime-builder"})
        with urllib.request.urlopen(request, timeout=180) as response, staged.open("wb") as output:
            shutil.copyfileobj(response, output)
        if digest(staged) != item["sha256"]:
            raise ValueError(f"upstream digest mismatch: {item['name']}")
        staged.replace(path)
    if digest(path) != item["sha256"]:
        raise ValueError(f"cached digest mismatch: {item['name']}")
    return path


def extract(archive, destination):
    destination.mkdir(parents=True, exist_ok=False)
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as z:
            for raw in z.namelist():
                if PurePosixPath(raw).is_absolute() or ".." in PurePosixPath(raw).parts or "\\" in raw or ":" in raw:
                    raise ValueError(f"unsafe archive entry: {raw}")
            z.extractall(destination)
    else:
        with tarfile.open(archive) as t:
            t.extractall(destination, filter="data")
    entries = list(destination.iterdir())
    return entries[0] if len(entries) == 1 and entries[0].is_dir() else destination


def licenses_from(archive, output):
    """Keep upstream license/notices verbatim; full archives ship beside the binary ZIP."""
    def wanted(raw):
        name = PurePosixPath(raw).name.lower()
        return (name.startswith(("license", "copying", "copyright", "notice", "patents"))
                or name in ("authors", "readme.ijg"))
    output.mkdir(parents=True, exist_ok=True)
    saved = 0
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as z:
            files = [(n, z.read(n)) for n in z.namelist() if not n.endswith("/") and wanted(n)]
    else:
        with tarfile.open(archive) as t:
            files = [(n.name, t.extractfile(n).read()) for n in t if n.isfile() and wanted(n.name)]
    for raw, content in files:
        # Preserve distinct notices even if many subprojects call them LICENSE.
        name = hashlib.sha256(raw.encode()).hexdigest()[:16] + "-" + PurePosixPath(raw).name[:80]
        (output / name).write_bytes(content)
        saved += 1
    return saved


def prepare(build_dir):
    lock = json.loads(LOCK.read_text())
    if lock["schemaVersion"] != 1 or lock["target"] != TARGET:
        raise ValueError("invalid engine lock")
    cargo_lock = ROOT / "scripts/windows-pngquant.Cargo.lock"
    if digest(cargo_lock) != lock["pngquantCargoLockSha256"]:
        raise ValueError("pngquant Cargo.lock does not match engine lock")
    downloads = build_dir / "windows-downloads"
    downloads.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=6) as pool:
        list(pool.map(lambda item: download(item, downloads), lock["archives"]))
    # An interrupted build is never reused as a prepared runtime.
    work = Path(tempfile.mkdtemp(prefix="windows-runtime-", dir=build_dir))
    sources, paths = work / "sources", {}
    sources.mkdir()
    for item in lock["archives"]:
        if item["kind"] != "corresponding-source" or item["name"] == "vips-source-librsvg":
            paths[item["name"]] = str(extract(downloads / item["archive"], sources / item["name"]))
    shutil.copytree(paths["imagequant"], Path(paths["pngquant"]) / "lib", dirs_exist_ok=True)
    shutil.copy2(cargo_lock, Path(paths["pngquant"]) / "Cargo.lock")
    write_json(work / "source-paths.json", paths)
    if os.name != "nt":
        raise RuntimeError(f"Sources verified in {work}; building requires Windows x64 MSVC")
    subprocess.run(["pwsh", "-NoProfile", "-File", str(ROOT / "scripts/build-windows.ps1"),
                    "-WorkDir", str(work)], check=True)
    prepared = {"work": str(work), "lockSha256": digest(LOCK), "sources": paths}
    write_json(build_dir / "windows-prepared.json", prepared)
    print(json.dumps(prepared))


def zip_directory(source, destination):
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for path in sorted(source.rglob("*")):
            if path.is_file():
                if path.is_symlink():
                    raise ValueError(f"refusing symlink: {path}")
                z.write(path, path.relative_to(source).as_posix())


def package(build_dir, output):
    lock = json.loads(LOCK.read_text())
    prepared = json.loads((build_dir / "windows-prepared.json").read_text())
    if prepared["lockSha256"] != digest(LOCK):
        raise ValueError("prepared engines use another lock")
    work = Path(prepared["work"])
    paths = {k: Path(v) for k, v in prepared["sources"].items()}
    output.mkdir(parents=True, exist_ok=True)
    stage = output / "candidate"
    stage.mkdir(exist_ok=False)
    (stage / "runtime/bin").mkdir(parents=True)
    (stage / "runtime/empty-modules").mkdir()
    # Keep upstream bin/plugin/etc/share layout, omit development headers/import libs.
    for name in ["bin", "etc", "share"]:
        if (paths["vips"] / name).exists():
            shutil.copytree(paths["vips"] / name, stage / "runtime/vips" / name)
    for name in TOOLS:
        source, relative = TOOLS[name]
        if source != "vips":
            candidate = (paths["oxipng"] / "oxipng.exe") if name == "oxipng" else work / "bin" / (name + ".exe")
            shutil.copy2(candidate, stage / relative)
    shutil.copy2(work / "bin/vcruntime140.dll", stage / "runtime/bin/vcruntime140.dll")
    shutil.copytree(work / "msvc-licenses", stage / "licenses/msvc")
    shutil.copy2(work / "build-tools.json", stage / "licenses/build-tools.json")
    shutil.copy2(build_dir / "release/yali.exe", stage / "yali.exe")
    shutil.copy2(build_dir / "release/yali-app.exe", stage / "Yali-workbench.exe")
    shutil.copy2(ROOT / "LICENSE", stage / "LICENSE")
    items = {item["name"]: item for item in lock["archives"]}
    for item in lock["archives"]:
        licenses_from(build_dir / "windows-downloads" / item["archive"], stage / "licenses" / item["name"])
    vendor = work / "vendor"
    if not vendor.is_dir() or not (work / "vendor-librsvg").is_dir():
        raise ValueError("missing Cargo corresponding sources")
    for manifest in list(vendor.glob("*/Cargo.toml")) + list((work / "vendor-librsvg").glob("*/Cargo.toml")):
        for path in manifest.parent.rglob("*"):
            if path.is_file() and path.name.lower().startswith(("license", "copying", "copyright", "notice")):
                dest = stage / "licenses/cargo" / manifest.parent.name / path.relative_to(manifest.parent)
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, dest)
    # Source bundle contains all original archives, Rust vendored dependencies and exact build recipes.
    source_stage = output / "sources"
    source_stage.mkdir()
    for item in lock["archives"]:
        if item["kind"] != "binary":
            shutil.copy2(build_dir / "windows-downloads" / item["archive"], source_stage / item["archive"])
    shutil.copytree(vendor, source_stage / "cargo-vendor")
    shutil.copytree(work / "vendor-librsvg", source_stage / "cargo-vendor-librsvg")
    (source_stage / "scripts").mkdir()
    for name in ["windows-runtime.py", "build-windows.ps1", "windows-engines.lock.json", "windows-pngquant.Cargo.lock"]:
        shutil.copy2(ROOT / "scripts" / name, source_stage / "scripts" / name)
    (source_stage / "README-Sources.txt").write_text(
        "Corresponding source for the Yali Windows image runtime\n\n"
        "Original source archives and expected hashes are listed in scripts/windows-engines.lock.json.\n"
        "MSVC image tools: prepare extracts sources, places imagequant under pngquant/lib, copies\n"
        "windows-pngquant.Cargo.lock, then invokes build-windows.ps1 (CMake/NMake/Cargo, static CRT).\n"
        "Rebuild requires Windows x64 MSVC/SDK, CMake, Ninja, Python 3.13 and Rust 1.100.0.\n"
        "For offline Cargo rebuilds use cargo-vendor and cargo-vendor-librsvg as directory sources.\n"
        "Original libvips build recipes/patches are in vips-build.tar.gz and mxe.tar.gz.\n"
        "Use the upstream v8.18.7 build script for libvips-all x86_64 shared with its versioned sources.\n"
        "libvips DLLs remain dynamically replaceable; do not modify the Yali data directory.\n",
        encoding="utf-8")
    source_zip = output / "Yali_windows_corresponding-sources.zip"
    zip_directory(source_stage, source_zip)
    write_json(stage / "licenses/SOURCES.json", {
        "sources": lock["archives"], "sourceBundle": source_zip.name, "sha256": digest(source_zip),
        "notice": "Separate executables communicate via subprocesses. Third-party licenses remain applicable. "
                  "Source bundle includes upstream source, MXE patches/recipes and Cargo dependencies; "
                  "dynamic libvips libraries may be replaced/rebuilt. Yali is Apache-2.0.",
    })
    (stage / "README-Windows.txt").write_text(
        "鸭梨 Windows 11 x64 实验候选\n\n"
        "启动 Yali-workbench.exe；CLI 使用 yali.exe --help。\n"
        "需要已安装的 Microsoft Edge WebView2 Evergreen Runtime，不会静默安装。\n"
        "官方安装：https://developer.microsoft.com/microsoft-edge/webview2/\n"
        "图片工具随包分发。解压后可移动，勿单独移动 EXE 或 runtime。\n"
        "数据默认 USERPROFILE/.yali；YALI_HOME 绝对路径可显式覆盖。\n"
        "旧 HOME/.yali 不自动迁移，可用 YALI_HOME 指向原目录。\n"
        "支持 JPEG/PNG/GIF/WebP/BMP/TIFF 输入，JPEG/PNG/WebP 输出；GIF 默认保持动画。\n"
        "多页 TIFF、动画转静态、APNG 缩放和动画 WebP 重编码会明确拒绝。\n"
        "无 FFmpeg 仍可处理和视觉对比，仅 SSIM 不可用。\n"
        "GUI、WebView2 缺失及 SmartScreen 表现需 Windows 11 实机人工验收；无签名。\n"
        "第三方许可见 licenses，同一 artifact 提供对应源码 ZIP；不要遗漏源码资产。\n", encoding="utf-8")
    version = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())["version"]
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    dirty = bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip())
    info = dict(version=version, commit=commit, target=TARGET, sourceDirty=dirty,
                repository=os.environ.get("GITHUB_REPOSITORY", ""),
                runUrl=f"https://github.com/{os.environ.get('GITHUB_REPOSITORY', '')}/actions/runs/{os.environ.get('GITHUB_RUN_ID', '')}",
                imageEnginesBundled=True, guiManuallyVerified=False,
                validation={"rustTests": "see Actions test step", "cliSmoke": "see companion smoke-report.json", "windows11Gui": "pending"})
    write_json(stage / "build-info.json", info)
    tool_map = {}
    for name, (source, relative) in TOOLS.items():
        tool_map[name] = dict(path=relative, version=items[source]["version"], source=items[source]["url"],
                              license="licenses/SOURCES.json", noticeDirectory=f"licenses/{source}")
    manifest = dict(schemaVersion=1, target=TARGET, tools=tool_map, engineLockSha256=digest(LOCK),
                    files={p.relative_to(stage).as_posix(): digest(p) for p in sorted(stage.rglob("*")) if p.is_file()})
    write_json(stage / "yali-runtime.json", manifest)
    report = check(stage)
    write_json(output / "dependency-report.json", report)
    archive = output / f"Yali_{version}_windows_x86_64-experiment.zip"
    zip_directory(stage, archive)
    unpacked = output / "解压 验证 [Yali]"  # final ZIP smoke uses this path, not staging input
    extract(archive, unpacked)
    check(unpacked)
    (output / "SHA256SUMS").write_text(
        "\n".join(f"{digest(p)}  {p.name}" for p in [archive, source_zip]) + "\n", encoding="utf-8")
    shutil.copy2(stage / "build-info.json", output / "build-info.json")
    print(f"Verified ZIP: {archive}\nUnpacked: {unpacked}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ["prepare", "package"]:
        p = sub.add_parser(command)
        p.add_argument("--build-dir", type=Path, default=ROOT / "target")
        if command == "package":
            p.add_argument("--output", type=Path, default=ROOT / "target/windows-assets")
    p = sub.add_parser("check")
    p.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "check":
        print(json.dumps(check(args.directory), indent=2))
    elif args.command == "prepare":
        prepare(args.build_dir.resolve())
    else:
        package(args.build_dir.resolve(), args.output.resolve())


if __name__ == "__main__":
    main()
