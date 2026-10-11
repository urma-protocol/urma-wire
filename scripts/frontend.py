#!/usr/bin/env python3
"""Run Trunk for Tauri with its documented boolean NO_COLOR format."""
import os
import sys


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ("serve", "build"):
        raise SystemExit("usage: frontend.py {serve|build} [trunk options]")
    env = os.environ.copy()
    # Common shell NO_COLOR=1 is valid conventionally but Trunk parses a bool.
    if "NO_COLOR" in env:
        env["NO_COLOR"] = "false" if env["NO_COLOR"].lower() in ("0", "false") else "true"
    os.execvpe("trunk", ["trunk", *sys.argv[1:]], env)


if __name__ == "__main__":
    main()
