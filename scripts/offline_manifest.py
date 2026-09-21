"""Trunk post-build: cover every emitted asset, including wasm-bindgen snippets."""
import hashlib
import json
import os
from pathlib import Path

root = Path(os.environ["TRUNK_STAGING_DIR"])
files = sorted(p for p in root.rglob("*") if p.is_file() and p.name != "sw.js")
manifest = [{"url": "/" + p.relative_to(root).as_posix(),
             "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in files]
version = hashlib.sha256(json.dumps(manifest).encode()).hexdigest()
template = Path("web/sw.js").read_text()
(root / "sw.js").write_text("const VERSION = " + json.dumps(version) + ";\nconst ASSETS = " +
                            json.dumps(manifest) + ";\n" + template)
