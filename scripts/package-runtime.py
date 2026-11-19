#!/usr/bin/env python3
"""构建期收集 Mach-O 依赖、重定位并签名；运行期不依赖 Python/Homebrew。"""
import argparse
from functools import lru_cache
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent.parent
ENGINES = {
    "cjpeg": "mozjpeg", "jpegtran": "mozjpeg", "pngquant": "pngquant",
    "oxipng": "oxipng", "gifsicle": "gifsicle", "cwebp": "webp",
    "gif2webp": "webp", "avifenc": "libavif", "cjxl": "jpeg-xl",
    "vips": "vips", "vipsthumbnail": "vips", "ffmpeg": "ffmpeg",
}
LOADS = {"LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LOAD_UPWARD_DYLIB"}


def run(*args):
    result = subprocess.run([str(a) for a in args], text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f"{' '.join(map(str, args))}\n{result.stderr.strip()}")
    return result.stdout


def system(path):
    return path.startswith(("/usr/lib/", "/System/Library/"))


@lru_cache(maxsize=None)
def macho(path):
    info = {"loads": [], "rpaths": [], "id": False, "minimum": "11.0"}
    for block in re.split(r"Load command \d+\n", run("/usr/bin/otool", "-l", path)):
        cmd = re.search(r"\bcmd (LC_\w+)", block)
        if not cmd:
            continue
        cmd = cmd[1]
        if cmd in LOADS:
            info["loads"].append(re.search(r"\bname (.+?) \(offset", block)[1])
        elif cmd == "LC_ID_DYLIB":
            info["id"] = True
        elif cmd == "LC_RPATH":
            info["rpaths"].append(re.search(r"\bpath (.+?) \(offset", block)[1])
        elif cmd in ("LC_BUILD_VERSION", "LC_VERSION_MIN_MACOSX"):
            value = re.search(r"(?:minos|version) (\d+(?:\.\d+)+)", block)
            if value:
                info["minimum"] = value[1]
    return info


def resolve_dependency(value, source, rpaths):
    if value.startswith("@loader_path/"):
        candidates = [source.parent / value.removeprefix("@loader_path/")]
    elif value.startswith("@rpath/"):
        candidates = []
        for rpath in rpaths:
            rpath = rpath.replace("@loader_path", str(source.parent)).replace("@executable_path", str(source.parent))
            candidates.append(Path(rpath) / value.removeprefix("@rpath/"))
    elif value.startswith("@executable_path/"):
        candidates = [source.parent / value.removeprefix("@executable_path/")]
    else:
        candidates = [Path(value)]
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    raise RuntimeError(f"无法解析 {source.name} 的动态依赖 {value}；检查构建机引擎安装")


def brew_binary(package, name):
    for prefix in (Path("/opt/homebrew"), Path("/usr/local")):
        candidate = prefix / "opt" / package / "bin" / name
        if candidate.is_file():
            return candidate.resolve()
    raise RuntimeError(f"构建机缺少 {package}/{name}，请先按 Brewfile 安装依赖")


def version(value):
    return tuple(map(int, value.split("."))) + (0,) * (3 - len(value.split(".")))


def prepare(build_dir):
    stage = build_dir / "yali-bundle-input"
    if stage.exists():
        # 只清理此脚本明确管理的构建暂存目录。
        if not (stage / ".yali-stage").is_file():
            raise RuntimeError(f"拒绝覆盖非托管目录 {stage}")
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    (stage / ".yali-stage").write_text("T-023\n")
    target = re.search(r"^host: (.+)$", run("rustc", "-vV"), re.M)[1]
    (stage / "bin").mkdir()
    (stage / "Frameworks").mkdir()
    roots = {"yali": (build_dir / "release/yali").resolve()}
    roots.update({name: brew_binary(pkg, name) for name, pkg in ENGINES.items()})
    sources = {src: stage / "bin" / f"{name}-{target}" for name, src in roots.items()}
    # 只携带产品图片格式所需插件，不引入 PDF/Whole-slide/ImageMagick 插件。
    vips_lib = roots["vips"].parent.parent / "lib"
    module_dirs = sorted(vips_lib.glob("vips-modules-*"))
    if len(module_dirs) != 1:
        raise RuntimeError("无法唯一确定 libvips 模块 ABI 目录")
    module_dir = module_dirs[0]
    modules = []
    for name in ("vips-heif.dylib", "vips-jxl.dylib"):
        src = (module_dir / name).resolve(strict=True)
        # 代码目录名避免句点，否则 codesign 会把它误判为独立 bundle。
        dest = stage / "Frameworks" / "VipsModules" / name
        sources[src] = dest
        modules.append(str(dest.relative_to(stage / "Frameworks")))
    pending = list(sources)
    edges = {}
    names = {}
    packages = set()
    minimum = "14.0"
    while pending:
        source = pending.pop()
        info = macho(source)
        minimum = max(minimum, info["minimum"], key=version)
        if run("/usr/bin/lipo", "-archs", source).strip() != target.split("-")[0].replace("aarch64", "arm64"):
            raise RuntimeError(f"引擎架构与 Rust 构建目标不一致：{source}")
        edges[source] = []
        if "Cellar" in source.parts:
            i = source.parts.index("Cellar")
            packages.add(Path(*source.parts[:i + 3]))
        for value in info["loads"]:
            if system(value):
                continue
            dependency = resolve_dependency(value, source, info["rpaths"])
            edges[source].append((value, dependency))
            if dependency not in sources:
                name = dependency.name
                if name in names and names[name] != dependency:
                    raise RuntimeError(f"动态库重名，不能安全合并：{names[name]} / {dependency}")
                names[name] = dependency
                sources[dependency] = stage / "Frameworks" / name
                pending.append(dependency)
    for source, dest in sources.items():
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, dest)
        dest.chmod(dest.stat().st_mode | 0o200)
        info = macho(source)
        args = []
        if info["id"]:
            args += ["-id", f"@rpath/{dest.name}"]
        for old, dependency in edges[source]:
            relative = os.path.relpath(sources[dependency], dest.parent)
            args += ["-change", old, f"@loader_path/{relative}"]
        for rpath in info["rpaths"]:
            args += ["-delete_rpath", rpath]
        if args:
            run("/usr/bin/install_name_tool", *args, dest)
        run("/usr/bin/codesign", "--force", "--sign", "-", "--timestamp=none", dest)
    notices = stage / "third-party"
    notices.mkdir()
    inventory = []
    for package in sorted(packages):
        dest = notices / package.parent.name / package.name
        dest.mkdir(parents=True)
        for pattern in ("LICENSE*", "COPYING*", "NOTICE*", "AUTHORS*", "INSTALL_RECEIPT.json", "sbom.spdx.json", ".brew/*.rb"):
            for file in package.glob(pattern):
                if file.is_file():
                    shutil.copy2(file, dest / file.name)
        licenses = package / "share/licenses"
        if licenses.is_dir():
            shutil.copytree(licenses, dest / "licenses", dirs_exist_ok=True)
        inventory.append({"name": package.parent.name, "version": package.name})
    (notices / "README.txt").write_text(
        "Bundled third-party components, their upstream licenses, Homebrew build recipes, receipts and SBOMs.\n"
        "Upstream source URLs and build options are recorded in the recipes/SBOMs.\n"
        "This personal build is not an Apple-notarized release.\n", encoding="utf-8")
    manifest = {
        "schemaVersion": 1, "target": target, "minimumSystemVersion": minimum,
        "executables": sorted(roots),
        "libraries": sorted(str(p.relative_to(stage / "Frameworks")) for p in sources.values() if p.is_relative_to(stage / "Frameworks")),
        "vipsModules": modules, "vipsModuleDirectory": module_dir.name, "packages": inventory,
    }
    (stage / "yali-runtime.json").write_text(json.dumps(manifest, indent=2) + "\n")
    config = {"bundle": {
        "externalBin": [str(stage / "bin" / name) for name in roots],
        "resources": {str(stage / "yali-runtime.json"): "yali-runtime.json", str(notices): "third-party/"},
        "macOS": {"minimumSystemVersion": minimum},
    }}
    (stage / "tauri-bundle.json").write_text(json.dumps(config, indent=2) + "\n")
    print(f"✓ 运行时：{len(roots)} 个可执行文件、{len(manifest['libraries'])} 个动态库/插件，最低 macOS {minimum}")


def finalize(build_dir, app):
    contents = app / "Contents"
    stage = build_dir / "yali-bundle-input"
    shutil.copytree(stage / "Frameworks", contents / "Frameworks", dirs_exist_ok=True)
    vips = contents / "Resources/vips"
    vips.mkdir(parents=True, exist_ok=True)
    # libvips 要求固定 ABI 目录名；Resources 中仅保留指向标准代码目录的链接。
    lib = vips / "lib"
    lib.mkdir(exist_ok=True)
    manifest = json.loads((stage / "yali-runtime.json").read_text())
    (lib / manifest["vipsModuleDirectory"]).symlink_to("../../../Frameworks/VipsModules", target_is_directory=True)
    (contents / "Resources/empty-modules").mkdir(exist_ok=True)
    for file in (contents / "Frameworks").rglob("*"):
        if file.is_file():
            run("/usr/bin/codesign", "--force", "--sign", "-", "--timestamp=none", file)
    for file in (contents / "MacOS").iterdir():
        if file.is_file() and file.name != "yali-app":
            run("/usr/bin/codesign", "--force", "--sign", "-", "--timestamp=none", file)
    run("/usr/bin/codesign", "--force", "--sign", "-", "--timestamp=none", "--identifier", "app.yali", app)
    check(app)


def check(app):
    contents = (app / "Contents").resolve()
    manifest = json.loads((contents / "Resources/yali-runtime.json").read_text())
    paths = [contents / "MacOS" / name for name in ["yali-app", *manifest["executables"]]]
    paths += [contents / "Frameworks" / name for name in manifest["libraries"]]
    for path in paths:
        if not path.is_file():
            raise RuntimeError(f"App 缺少 {path.relative_to(contents)}")
        info = macho(path)
        for rpath in info["rpaths"]:
            if not rpath.startswith(("@loader_path", "@executable_path")):
                raise RuntimeError(f"App 残留外部 RPATH：{path.name}: {rpath}")
        for value in info["loads"]:
            if system(value):
                continue
            resolved = resolve_dependency(value, path, info["rpaths"])
            if not resolved.is_relative_to(contents):
                raise RuntimeError(f"App 引用了外部动态库：{path.name}: {value}")
    with (contents / "Info.plist").open("rb") as file:
        plist = plistlib.load(file)
    if version(plist["LSMinimumSystemVersion"]) < version(manifest["minimumSystemVersion"]):
        raise RuntimeError("App 最低系统声明低于内嵌引擎要求")
    run("/usr/bin/codesign", "--verify", "--deep", "--strict", app)
    print(f"✓ {len(paths)} 个 Mach-O 的动态依赖全部位于 App 或系统目录；ad-hoc 签名通过")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "finalize", "check"))
    parser.add_argument("--build-dir", type=Path, default=ROOT / "target")
    parser.add_argument("--app", type=Path)
    args = parser.parse_args()
    build_dir = args.build_dir.resolve()
    app = (args.app or build_dir / "release/bundle/macos/Yali.app").resolve()
    if args.action == "prepare":
        prepare(build_dir)
    elif args.action == "finalize":
        finalize(build_dir, app)
    else:
        check(app)


if __name__ == "__main__":
    main()
