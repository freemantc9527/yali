#!/usr/bin/env python3
"""Read-only loader probes for a failed Windows test process."""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--build-dir", type=Path, required=True)
args = parser.parse_args()
if os.name != "nt":
    parser.error("Windows loader required")
engine_dirs = {os.path.normcase(os.path.normpath(path))
               for path in os.environ.get("YALI_ENGINES_PATH", "").split(os.pathsep) if path}
clean = dict(os.environ)
clean["PATH"] = os.pathsep.join(path for path in os.environ.get("PATH", "").split(os.pathsep)
                              if os.path.normcase(os.path.normpath(path)) not in engine_dirs)
for binary in args.build_dir.glob("debug/build/yali-app/*/out/yali_app_lib*.exe"):
    for label, environment in [("injected-engine-path", os.environ), ("without-engine-path", clean)]:
        result = subprocess.run([str(binary.resolve()), "--list"], env=environment, capture_output=True,
                                encoding="utf-8", errors="replace", timeout=30)
        print(json.dumps({"binary": str(binary), "environment": label,
                          "returncode": result.returncode, "hex": hex(result.returncode & 0xffffffff),
                          "stdout": result.stdout[:5000], "stderr": result.stderr[:2000]}, ensure_ascii=False), flush=True)
    print(json.dumps({"engineDlls": [str(dll) for raw in os.environ.get("YALI_ENGINES_PATH", "").split(os.pathsep)
                                    if raw for dll in Path(raw).glob("*.dll")]}, ensure_ascii=False), flush=True)
