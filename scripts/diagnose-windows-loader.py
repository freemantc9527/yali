#!/usr/bin/env python3
"""Read-only loader probes for a failed Windows test process."""
import argparse
import ctypes
import struct
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

def imports(path):
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 60)[0]
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    sections = optional + optional_size
    def rva(value):
        for i in range(count):
            size, address, raw_size, raw_offset = struct.unpack_from("<IIII", data, sections + i * 40 + 8)
            if address <= value < address + max(size, raw_size):
                return raw_offset + value - address
        return value
    def string(offset):
        return data[offset:data.index(b"\0", offset)].decode("ascii")
    address = struct.unpack_from("<I", data, optional + 120)[0]
    if not address:
        return {}
    offset, result = rva(address), {}
    while any(struct.unpack_from("<5I", data, offset)):
        thunk, _, _, dll, first = struct.unpack_from("<5I", data, offset)
        names, entry = [], rva(thunk or first)
        while True:
            value = struct.unpack_from("<Q", data, entry)[0]
            if not value:
                break
            names.append(value & 0xffff if value & (1 << 63) else string(rva(value) + 2))
            entry += 8
        result[string(rva(dll))] = names
        offset += 20
    return result


kernel = ctypes.WinDLL("kernel32", use_last_error=True)
kernel.GetProcAddress.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
kernel.GetProcAddress.restype = ctypes.c_void_p
for binary in args.build_dir.glob("debug/build/yali-app/*/out/yali_app_lib*.exe"):
    dependencies = imports(binary)
    for dll, symbols in dependencies.items():
        try:
            loaded = ctypes.WinDLL(dll)
        except OSError as error:
            print(json.dumps({"dll": dll, "loadError": str(error), "symbols": symbols}, ensure_ascii=False), flush=True)
            continue
        missing = []
        for symbol in symbols:
            pointer = ctypes.c_void_p(symbol) if isinstance(symbol, int) else ctypes.cast(ctypes.c_char_p(symbol.encode("ascii")), ctypes.c_void_p)
            if not kernel.GetProcAddress(loaded._handle, pointer):
                missing.append(symbol)
        print(json.dumps({"dll": dll, "missingSymbols": missing, "symbolCount": len(symbols)}, ensure_ascii=False), flush=True)
for binary in args.build_dir.glob("debug/build/yali-app/*/out/yali_app_lib*.exe"):
    for label, environment in [("injected-engine-path", os.environ), ("without-engine-path", clean)]:
        result = subprocess.run([str(binary.resolve()), "--list"], env=environment, capture_output=True,
                                encoding="utf-8", errors="replace", timeout=30)
        print(json.dumps({"binary": str(binary), "environment": label,
                          "returncode": result.returncode, "hex": hex(result.returncode & 0xffffffff),
                          "stdout": result.stdout[:5000], "stderr": result.stderr[:2000]}, ensure_ascii=False), flush=True)
    print(json.dumps({"engineDlls": [str(dll) for raw in os.environ.get("YALI_ENGINES_PATH", "").split(os.pathsep)
                                    if raw for dll in Path(raw).glob("*.dll")]}, ensure_ascii=False), flush=True)
