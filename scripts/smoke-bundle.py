#!/usr/bin/env python3
"""在禁止访问 Homebrew、源码与原账户数据的沙盒中验证最终 App。"""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, default=ROOT / "target/release/bundle/macos/Yali.app")
    parser.add_argument("--keep", action="store_true", help="保留隔离副本，供原生 GUI 验收")
    args = parser.parse_args()
    work = Path(tempfile.mkdtemp(prefix="yali-standalone-", dir="/private/tmp")).resolve()
    home = work / "Home & O'Neil"
    app = home / "Applications/Yali.app"
    app.parent.mkdir(parents=True)
    shutil.copytree(args.app.resolve(), app, symlinks=True)
    inputs = work / "inputs"
    shutil.copytree(ROOT / "benchkit/samples", inputs)
    (work / "tmp").mkdir()
    env = {"HOME": str(home), "YALI_HOME": str(home / ".yali"), "TMPDIR": str(work / "tmp"),
           "PATH": "/usr/bin:/bin", "LANG": "en_US.UTF-8"}
    profile = work / "no-development-dependencies.sb"
    denied = ["/opt/homebrew", "/usr/local", str(ROOT), str(Path.home())]
    profile.write_text('(version 1)\n(allow default)\n' + '\n'.join(
        f'(deny file-read* file-write* (subpath {json.dumps(p)}))' for p in denied) + '\n')
    cli = app / "Contents/MacOS/yali"

    def run(*command, include_stderr=False, isolated=True):
        invocation = [str(c) for c in command]
        if isolated:
            invocation = ["/usr/bin/sandbox-exec", "-f", str(profile), *invocation]
        result = subprocess.run(invocation,
                                cwd=work, env=env, text=True, capture_output=True, timeout=180)
        if result.returncode:
            raise RuntimeError(f"命令失败 {command}\n{result.stdout}\n{result.stderr}")
        return result.stdout + (result.stderr if include_stderr else "")

    def reports(*command):
        result = [json.loads(s) for s in run(cli, *command).splitlines() if s.startswith("{")]
        if not result:
            raise RuntimeError(f"CLI 没有返回处理结果：{command}")
        for report in result:
            if report["status"] == "done":
                output = Path(report["output"])
                assert output.is_file() and output.stat().st_size > 0, report
            else:
                assert report["status"] == "larger_output", report
        return result

    def service_status(action):
        output = run(cli, "services", action)
        try:
            return json.loads(output)
        except json.JSONDecodeError:
            print(f"Services 原始输出：{output!r}", flush=True)
            raise

    try:
        # 用实际读文件证实沙盒禁令生效，而不只是删 PATH。
        for path in ["/opt/homebrew/bin/cwebp", str(ROOT / "README.md")]:
            probe = subprocess.run(["/usr/bin/sandbox-exec", "-f", str(profile), "/bin/cat", path],
                                   cwd=work, env=env, capture_output=True)
            assert probe.returncode != 0, f"沙盒没有隔离 {path}"
        doctor = run(cli, "doctor")
        assert not any(line.startswith("✗") for line in doctor.splitlines()), doctor
        for line in doctor.splitlines():
            if line.startswith("✓") and "Sips:" not in line:
                assert str(app / "Contents/MacOS") in line, line
        print("✓ Homebrew、源码和原账户数据不可读，doctor 引擎全部来自 App（sips 为系统组件）", flush=True)
        poison = work / "poison-engines"
        poison.mkdir()
        (poison / "cjpeg").write_text("#!/bin/sh\nexit 99\n")
        (poison / "cjpeg").chmod(0o755)
        env["YALI_ENGINES_PATH"] = str(poison)
        assert str(poison) not in run(cli, "doctor"), "包内运行时被外部引擎覆盖"
        del env["YALI_ENGINES_PATH"]
        assert set(run(cli, "profiles", "list").splitlines()) == {"default", "jpg", "thumbnail", "archive"}
        samples = sorted(p for p in inputs.iterdir() if not p.name.startswith("."))
        results = reports("compress", *samples, "--profile", "default")
        assert len(results) == 6
        for preset, source in [("jpg", "transparent.png"), ("thumbnail", "photo.jpg"), ("archive", "photo.jpg")]:
            result = reports("compress", inputs / source, "--profile", preset)[0]
            assert result["status"] == "done", result
            if preset == "thumbnail":
                size = run("/usr/bin/sips", "-g", "pixelWidth", "-g", "pixelHeight", result["output"])
                assert "pixelWidth: 1024" in size and "pixelHeight: 683" in size, size
        print("✓ 6 张金样与四个预设真实处理，1024×683 缩小尺寸通过", flush=True)
        for fmt in ("webp", "avif", "jxl"):
            result = reports("convert", inputs / "photo.jpg", "--to", fmt)[0]
            assert result["status"] == "done", result
        for fmt in ("tiff", "bmp"):
            dest = inputs / f"extra.{fmt}"
            run("/usr/bin/sips", "-s", "format", fmt, inputs / "photo.jpg", "--out", dest)
            assert reports("compress", dest, "--profile", "default")[0]["status"] == "done"
        print("✓ WebP / AVIF / JXL 输出及 TIFF / BMP 输入通过", flush=True)
        installed = service_status("install")
        assert installed["installed"], installed
        assert service_status("install")["installed"]
        services = home / "Library/Services"
        for title, source, preset in [("鸭梨 · 压一下", "photo.jpg", "default"), ("鸭梨 · 转 JPG", "ui.png", "jpg")]:
            workflow = services / f"{title}.workflow"
            with (workflow / "Contents/Info.plist").open("rb") as file:
                info = plistlib.load(file)
            if preset == "jpg":
                assert "public.jpeg" not in info["NSServices"][0]["NSSendFileTypes"]
            with (workflow / "Contents/document.wflow").open("rb") as file:
                document = plistlib.load(file)
            script = document["actions"][0]["action"]["ActionParameters"]["COMMAND_STRING"]
            isolated_copy = work / f"isolated-workflow-{preset}{Path(source).suffix}"
            shutil.copy2(inputs / source, isolated_copy)
            run("/bin/zsh", "-c", script, "yali-workflow", isolated_copy)
            assert list(work.glob(f"isolated-workflow-{preset}-*")), "隔离的 workflow shell 节点未生成图片"
            source_copy = work / f"workflow-{preset}{Path(source).suffix}"
            shutil.copy2(inputs / source, source_copy)
            # Automator.framework 明确拒绝从沙盒进程运行。shell 节点在上面强隔离，
            # 完整 workflow 在正常系统上下文执行；两者都只调用临时 App 内的 CLI。
            run("/usr/bin/automator", "-i", source_copy, workflow, isolated=False)
            assert list(work.glob(f"workflow-{preset}-*")), "workflow 未生成图片"
        print("✓ 两条 workflow shell 节点强隔离产图，完整 Automator workflow 在系统上下文产图；特殊路径通过", flush=True)
        assert not service_status("uninstall")["installed"]
        assert (home / ".yali/settings.json").exists() or (home / ".yali/profiles/default.json").exists()
        assert (home / ".yali/bin/yali").is_symlink()
        assert not list(services.glob("鸭梨*.workflow"))
        print("✓ 卸载只移除右键操作，预设和稳定 CLI 入口保留；FFmpeg 为可选外部能力", flush=True)
        # 保留模式留下已安装的动作，供随后原生 GUI 验证。
        if args.keep:
            run(cli, "services", "install")
            (work / "environment.json").write_text(json.dumps(env, ensure_ascii=False, indent=2))
        print(f"隔离验收通过：{work}", flush=True)
    except Exception:
        print(f"验收失败，保留现场：{work}", flush=True)
        raise
    else:
        if not args.keep:
            shutil.rmtree(work)


if __name__ == "__main__":
    main()
