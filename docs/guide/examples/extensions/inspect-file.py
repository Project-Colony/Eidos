#!/usr/bin/env python3
"""Minimal Eidos protocol v1 example; Python standard library only."""
import json
import pathlib
import sys

request = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
source = pathlib.Path(request["input"])
reply = {"protocol": 1, "request_id": request["request_id"]}
operation = request["operation"]
try:
    if operation == "preview":
        # This example handles UTF-8 text; Eidos already has native image viewers.
        with source.open("rb") as handle:
            data = handle.read(65537)
        text = data[:65536].decode("utf-8")
        if "\x00" in text:
            raise ValueError("Binary data is not supported by this text example")
        if len(data) > 65536:
            text += "\n[Example helper truncated the file at 64 KiB]"
        (pathlib.Path(request["workspace"]) / "preview.txt").write_text(text, encoding="utf-8")
        outcome = {"status": "handled", "result": {"kind": "preview", "path": "preview.txt"}}
    elif operation == "save_info":
        # A portable JSON save example with scalar top-level metadata.
        with source.open("rb") as handle:
            data = handle.read(65537)
        if len(data) > 65536:
            raise ValueError("This example accepts JSON saves up to 64 KiB")
        save = json.loads(data)
        if not isinstance(save, dict):
            raise ValueError("Expected a JSON object")
        fields = [{"label": str(key), "value": str(value)} for key, value in save.items()
                  if isinstance(value, (str, int, float, bool)) and str(value).strip()]
        outcome = {"status": "handled", "result": {"kind": "save_info", "fields": fields[:64],
                                                   "plugins": [], "light_plugins": []}}
    elif operation == "installer":
        # Only an explicitly marked extracted tree is handled; normal archives decline.
        root = pathlib.Path(request["source"])
        marker = root / "eidos-example.txt"
        if not marker.is_file():
            outcome = {"status": "declined"}
        elif "readme" not in request["answers"]:
            outcome = {"status": "prompt", "id": "readme", "title": "Install the example readme?",
                       "options": ["Install", "Cancel"], "multiple": False}
        elif request["answers"]["readme"] == [0]:
            outcome = {"status": "handled", "result": {"kind": "install", "files": [
                {"source": "eidos-example.txt", "destination": "docs/example.txt"}], "warnings": []}}
        else:
            outcome = {"status": "cancelled"}
    else:
        outcome = {"status": "declined"}
except (OSError, UnicodeError, ValueError) as error:
    outcome = {"status": "failed", "message": str(error)}
reply["outcome"] = outcome
print(json.dumps(reply, ensure_ascii=True))
