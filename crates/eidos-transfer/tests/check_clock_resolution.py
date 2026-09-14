"""Run the real transfer tests with colliding clock ticks on Linux."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

artifacts = subprocess.check_output([
    "cargo", "test", "-p", "eidos-transfer", "--tests", "--locked", "--no-run",
    "--message-format=json", *sys.argv[1:],
], text=True)
binaries = [item["executable"] for line in artifacts.splitlines()
            if (item := json.loads(line)).get("reason") == "compiler-artifact"
            and item.get("executable") and item["profile"]["test"]]
if not binaries:
    raise RuntimeError("Cargo produced no transfer test executables")

with tempfile.TemporaryDirectory(prefix="eidos-clock-check-") as scratch:
    library = Path(scratch) / "coarse-clock.so"
    subprocess.run([
        "cc", "-shared", "-fPIC", str(Path(__file__).with_name("coarse_clock.c")),
        "-ldl", "-o", str(library),
    ], check=True)
    env = os.environ.copy()
    env["LD_PRELOAD"] = ":".join(filter(None, (str(library), env.get("LD_PRELOAD"))))
    for binary in binaries:
        subprocess.run([binary], env=env, check=True, timeout=60)
