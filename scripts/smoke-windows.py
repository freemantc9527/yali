#!/usr/bin/env python3
"""Exercise the final extracted Windows ZIP offline, with isolated data and engine lookup."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile
import zlib

ROOT = Path(__file__).resolve().parent.parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png(path, color=6, width=96, height=64):
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[color]
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            rgb = ((x * 5 + y * 3) % 256, (x * 7) % 256, (y * 11) % 256)
            pixel = {0: [rgb[0]], 2: rgb, 3: [(x + y) % 2],
                     4: [rgb[0], 0 if x < 12 else 128],
                     6: [*rgb, 0 if x < 12 else 128]}[color]
            raw.extend(pixel[:channels])
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, color, 0, 0, 0))
    if color == 3:
        data += chunk(b"PLTE", b"\xff\0\0\0\0\xff") + chunk(b"tRNS", b"\0\xff")
    data += chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
    path.write_bytes(data)


def apng(path):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 6, 0, 0, 0))
    data += chunk(b"acTL", struct.pack(">II", 2, 0))
    for i, color in enumerate([b"\xff\0\0\xff", b"\0\0\xff\xff"]):
        control = struct.pack(">IIIIIHHBB", 0 if i == 0 else 1, 2, 2, 0, 0, 1, 10, 0, 0)
        data += chunk(b"fcTL", control)
        pixels = zlib.compress((b"\0" + color * 2) * 2)
        data += chunk(b"IDAT" if i == 0 else b"fdAT", pixels if i == 0 else struct.pack(">I", 2) + pixels)
    path.write_bytes(data + chunk(b"IEND", b""))


def gif_timing(path):
    data = path.read_bytes()
    offset, delays, loop = 13, [], None
    if data[10] & 128:
        offset += 3 * (2 ** ((data[10] & 7) + 1))
    def blocks(pos):
        values = []
        while data[pos]:
            size = data[pos]
            values.append(data[pos + 1:pos + 1 + size])
            pos += size + 1
        return pos + 1, values
    while offset < len(data):
        marker = data[offset]
        if marker == 0x3B:
            break
        if marker == 0x21:
            label = data[offset + 1]
            offset, values = blocks(offset + 2)
            if label == 0xF9:
                delays.append(struct.unpack_from("<H", values[0], 1)[0])
            if label == 0xFF and values[0].startswith(b"NETSCAPE"):
                loop = struct.unpack_from("<H", values[1], 1)[0]
        elif marker == 0x2C:
            flags = data[offset + 9]
            offset += 10
            if flags & 128:
                offset += 3 * (2 ** ((flags & 7) + 1))
            offset, _ = blocks(offset + 1)
        else:
            raise ValueError("invalid GIF block")
    return delays, loop


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--ffmpeg", type=Path, help="optional build-only quality tool, never added to ZIP")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("real Windows execution required; use test_windows_runtime.py for portable checks")
    directory = args.directory.resolve()
    manifest = json.loads((directory / "yali-runtime.json").read_text(encoding="utf-8"))
    report = {"result": "running", "automatic": [], "manual": [
        "Windows 11 clean user/VM: WebView2, GUI startup, drag/select, batch, profile persistence, compare, open/reveal",
        "No console flashing during processing; WebView2 missing behavior; unsigned EXE/SmartScreen",
        "Desktop OS differs from Windows Server Actions runner",
    ], "quality": []}
    # Every fixture, corruption, rename and cleanup stays inside this script's temporary directory.
    with tempfile.TemporaryDirectory(prefix="yali-smoke-") as temporary:
        work = Path(temporary)
        package = work / "移动 中文 & [Yali]"
        shutil.copytree(directory, package)
        user = work / "普通 用户"
        user.mkdir()
        inputs = work / "图片 & [输入]"
        inputs.mkdir()
        system = Path(os.environ.get("SystemRoot", "C:/Windows"))
        env = {k: v for k, v in os.environ.items() if k.upper() in {
            "SYSTEMROOT", "WINDIR", "TEMP", "TMP", "COMSPEC", "PATHEXT", "NUMBER_OF_PROCESSORS"}}
        env.update(USERPROFILE=str(user), YALI_HOME=str(user / ".yali"),
                   PATH=str(system / "System32") + ";" + str(system), LANG="en_US.UTF-8")
        assert "HOME" not in env
        cli = package / "yali.exe"
        tools = {name: package / tool["path"] for name, tool in manifest["tools"].items()}

        def run(program, *arguments, success=True, child_env=None):
            result = subprocess.run([str(program), *map(str, arguments)], cwd=work,
                                    env=child_env or env, capture_output=True, encoding="utf-8",
                                    errors="replace", timeout=180, creationflags=0x08000000)
            if success and result.returncode:
                raise RuntimeError(f"{program.name} {arguments}: {result.returncode}\n{result.stdout}\n{result.stderr}")
            return result

        def engine(name, *arguments, success=True):
            child = dict(env)
            child["PATH"] = ";".join(map(str, [tools[name].parent, package / "runtime/bin", system / "System32", system]))
            child["VIPSHOME"] = str(package / "runtime/vips")
            child["GIO_MODULE_DIR"] = str(package / "runtime/empty-modules")
            return run(tools[name], *arguments, success=success, child_env=child)

        def compress(path, preset="default", success=True):
            result = run(cli, "compress", path, "--profile", preset, success=success)
            rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
            if success:
                assert len(rows) == 1, result.stdout
                row = rows[0]
                assert row["status"] in ("done", "larger_output"), row
                if row["status"] == "done":
                    output = Path(row["output"])
                    assert output.is_file() and output.stat().st_size > 0, row
                    engine("vips", "avg", output)  # full decode, not only magic/header
                return row
            assert result.returncode != 0, result.stdout
            return result

        def save_profile(name, edit, success=True):
            profile = json.loads(run(cli, "profiles", "show", "default").stdout)
            profile["name"] = name
            edit(profile)
            path = work / (name + ".json")
            path.write_text(json.dumps(profile), encoding="utf-8")
            saved = run(cli, "profiles", "save", name, "--file", path, success=success)
            if not success:
                assert saved.returncode != 0, "invalid preset was accepted"

        for name, tool in manifest["tools"].items():
            flag = "-version" if name in ("cjpeg", "jpegtran", "cwebp", "gif2webp") else "--version"
            version = engine(name, flag)
            assert tool["version"] in version.stdout + version.stderr, (name, version.stdout, version.stderr)
        doctor = run(cli, "doctor").stdout
        for line in doctor.splitlines():
            if line.startswith("✓"):
                assert str(package) in line, line
        assert all(str(tools[name]) in doctor for name in ["cjpeg", "pngquant", "oxipng", "gifsicle", "cwebp", "gif2webp", "vipsthumbnail"]), doctor
        report["enginePaths"] = {name: str(path) for name, path in tools.items()}
        # A valid PE masquerading as an encoder would fail if the packaged resolver invoked it.
        poison = work / "fake-engines"
        poison.mkdir()
        for name in ["cjpeg", "pngquant", "vips", "vipsheader"]:
            shutil.copy2(cli, poison / (name + ".exe"))
        env["PATH"] = str(poison) + ";" + env["PATH"]
        env["YALI_ENGINES_PATH"] = str(poison)
        assert str(poison) not in run(cli, "doctor").stdout
        for name in ["photo.jpg", "ui.png", "palette.png", "transparent.png", "anim.gif"]:
            shutil.copy2(ROOT / "benchkit/samples" / name, inputs / ("中文 [1] " + name))
        photo = inputs / "中文 [1] photo.jpg"
        ui = inputs / "中文 [1] ui.png"
        gif = inputs / "中文 [1] anim.gif"
        bmp, tiff, webp = (inputs / name for name in ["图片 空格.bmp", "图片 空格.tiff", "图片 空格.webp"])
        engine("vips", "copy", photo, bmp)
        engine("vips", "copy", photo, tiff)
        engine("cwebp", photo, "-o", webp)
        originals = {p: sha(p) for p in inputs.iterdir()}
        rows = {}
        for source in [photo, ui, gif, bmp, tiff, webp]:
            rows[source] = compress(source)
            if source in (bmp, tiff):
                assert rows[source]["status"] == "done", rows[source]
                assert Path(rows[source]["output"]).suffix == (".png" if source == bmp else ".jpg")
        for source in [photo, ui, bmp, tiff, webp]:
            compress(source, "jpg")
            compress(source, "thumbnail")
        assert run(cli, "compress", gif, "--profile", "jpg", success=False).returncode != 0
        gif_result = rows[gif]
        gif_output = Path(gif_result["output"]) if gif_result["output"] else gif
        assert gif_timing(gif_output) == gif_timing(gif), "GIF frame/delay/loop changed"
        report["automatic"].append("six input formats; default/jpg/thumbnail; complete decode; GIF timing; no HOME; poison PATH")

        for color in [6, 4, 3]:
            source = inputs / f"透明 {color}.png"
            png(source, color=color)
            originals[source] = sha(source)
            row = compress(source, "jpg")
            assert row["status"] == "done", row
            point = engine("vips", "getpoint", row["output"], "0", "0").stdout
            assert all(float(v) >= 240 for v in point.split()), point
        alpha_webp, alpha_tiff = inputs / "透明.webp", inputs / "透明.tiff"
        engine("cwebp", "-lossless", inputs / "透明 6.png", "-o", alpha_webp)
        engine("vips", "copy", inputs / "透明 6.png", alpha_tiff)
        for source in [alpha_webp, alpha_tiff]:
            originals[source] = sha(source)
            row = compress(source, "jpg")
            point = engine("vips", "getpoint", row["output"], "0", "0").stdout
            assert all(float(v) >= 240 for v in point.split()), point
        report["automatic"].append("RGBA, grayscale alpha, palette tRNS, alpha WebP/TIFF to JPEG use white")

        save_profile("lossless-png", lambda p: p["quality"].update(kind="lossless"))
        lossless = compress(ui, "lossless-png")
        lossless_path = Path(lossless["output"]) if lossless["output"] else ui
        def rgba16(path, raw_name):
            raw_path = work / raw_name
            engine("vips", "rawsave", path, raw_path)
            bands = int(engine("vipsheader", "-f", "bands", path).stdout.strip())
            sample_format = engine("vipsheader", "-f", "format", path).stdout.strip()
            raw = raw_path.read_bytes()
            assert bands in (3, 4) and sample_format in ("uchar", "ushort")
            samples = raw if sample_format == "uchar" else [value[0] for value in struct.iter_unpack("=H", raw)]
            scale = 257 if sample_format == "uchar" else 1
            normalized = bytearray()
            for offset in range(0, len(samples), bands):
                pixel = [int(value) * scale for value in samples[offset:offset + bands]]
                if bands == 3:
                    pixel.append(65535)
                normalized.extend(struct.pack("<4H", *pixel))
            return normalized
        assert rgba16(ui, "a.raw") == rgba16(lossless_path, "b.raw"), "PNG lossless pixels changed"
        save_profile("manual", lambda p: p["quality"].update(kind="level", level=5))
        compress(photo, "manual")
        save_profile("target", lambda p: p["quality"].update(kind="targetBytes", level=None, targetBytes=1000, minQualityFloor=3))
        compress(photo, "target")
        save_profile("shrink", lambda p: p["resize"].update(maxWidth=32, maxHeight=32))
        small = compress(photo, "shrink")
        assert "width: 32" in engine("vipsheader", "-a", small["output"]).stdout
        for _ in range(3):
            run(cli, "profiles", "save", "manual", "--file", work / "manual.json")
            run(cli, "profiles", "show", "manual")
        run(cli, "settings", "reset")
        run(cli, "settings", "reset")
        run(cli, "settings", "show")
        report["automatic"].append("PNG lossless pixels; manual/target quality; shrink; repeated settings/profile save")

        animated = inputs / "动画.webp"
        engine("gif2webp", gif, "-o", animated)
        originals[animated] = sha(animated)
        compress(animated, success=False)
        ap = inputs / "动画.png"
        apng(ap)
        originals[ap] = sha(ap)
        ap_row = compress(ap)  # oxipng preserves APNG
        ap_output = Path(ap_row["output"]) if ap_row["output"] else ap
        data = ap_output.read_bytes()
        assert b"acTL" in data and data.count(b"fcTL") == 2, "APNG lost animation"
        compress(ap, "thumbnail", success=False)
        tall = work / "two-pages.png"
        engine("vips", "join", ui, ui, tall, "vertical")
        header = engine("vipsheader", "-f", "height", ui).stdout.strip()
        multi = inputs / "多页.tiff"
        engine("vips", "copy", tall, f"{multi}[page-height={header}]")
        originals[multi] = sha(multi)
        compress(multi, success=False)
        bad = inputs / "坏图.png"
        bad.write_bytes(b"\x89PNG\r\n\x1a\nbroken")
        mixed = run(cli, "compress", photo, bad, success=False)
        assert mixed.returncode != 0 and '"status"' in mixed.stdout
        report["automatic"].append("APNG/animated WebP/multipage TIFF guards; mixed-success batch")

        # Same-name aliases, read-only source and failed writes must not alter input bytes.
        save_profile("same-name", lambda p: p["output"].update(suffixTemplate=""), success=False)
        photo.chmod(0o444)
        try:
            compress(photo)
        finally:
            photo.chmod(0o666)
        assert all(sha(path) == original for path, original in originals.items())
        assert not list(inputs.glob("*.tmp.*")), "failed processing left staged output"
        # Fail closed for each removed mandatory EXE/DLL in a fresh process.
        for path in [tools["pngquant"], package / "runtime/bin/vcruntime140.dll"]:
            hidden = path.with_suffix(path.suffix + ".missing")
            path.rename(hidden)
            try:
                assert run(cli, "doctor", success=False).returncode != 0
                compress(photo, success=False)
            finally:
                hidden.rename(path)
        report["automatic"].append("input hashes; read-only input; same-name guard; missing EXE/DLL fail closed")
        # Check default data location separately without YALI_HOME/HOME.
        alternate = dict(env)
        alternate.pop("YALI_HOME")
        assert str(user / ".yali") in run(cli, "settings", "path", child_env=alternate).stdout
        undo_dir = work / "undo"
        undo_dir.mkdir()
        undo_source = undo_dir / "photo.jpg"
        shutil.copy2(photo, undo_source)
        undo_hash = sha(undo_source)
        undo_row = compress(undo_source, "jpg")
        run(cli, "undo", undo_dir)
        assert sha(undo_source) == undo_hash
        if undo_row["output"]:
            assert not Path(undo_row["output"]).exists()
        report["automatic"].append("USERPROFILE default data root; CLI undo; independent CWD; moved ZIP")

        if args.ffmpeg:
            ffmpeg = args.ffmpeg.resolve()
            def ssim(source, output):
                result = run(ffmpeg, "-hide_banner", "-i", source, "-i", output, "-lavfi", "ssim", "-f", "null", "-")
                return float(re.search(r"All:([0-9.]+)", result.stderr)[1])
            references = [
                (photo, "cjpeg", ["-quality", "75", "-outfile"], ".jpg"),
                (ui, "pngquant", ["--quality", "94-98", "--speed", "1", "--force", "--output"], ".png"),
                (webp, "cwebp", ["-q", "80"], ".webp"),
            ]
            for source, name, options, suffix in references:
                reference = work / ("reference-" + name + suffix)
                if name == "cwebp":
                    engine(name, *options, source, "-o", reference)
                else:
                    result = engine(name, *options, reference, source, success=False)
                    if result.returncode == 99 and name == "pngquant":
                        engine("oxipng", "-o", "2", "--strip", "safe", "--out", reference, source)
                    else:
                        assert result.returncode == 0, result.stderr
                row = compress(source)
                actual = Path(row["output"]) if row["output"] else source
                if reference.stat().st_size >= source.stat().st_size:
                    reference = source
                reference_ssim, actual_ssim = ssim(source, reference), ssim(source, actual)
                assert actual.stat().st_size <= reference.stat().st_size * 1.03
                assert actual_ssim >= reference_ssim * 0.97
                report["quality"].append(dict(engine=name, referenceSSIM=reference_ssim, actualSSIM=actual_ssim,
                                              sizeRatio=actual.stat().st_size / reference.stat().st_size,
                                              reference="same packaged version and parameters; macOS baseline unchanged"))
            report["automatic"].append("optional FFmpeg decoding and relative SSIM/size regression <=3%")
        else:
            report["quality"] = ["not run: --ffmpeg absent; basic processing verified without FFmpeg"]
        report["result"] = "passed"
    destination = directory.parent / "smoke-report.json"
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
