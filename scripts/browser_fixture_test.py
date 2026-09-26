"""Nonvisual local browser checks. Requires existing chromium + chromedriver.
Run after: NO_COLOR=true trunk build --features fixture-harness --offline
Uses only the committed public fixture and a disposable browser profile.
WIRE_DIST selects another build output, so a running `trunk serve` keeps its dist/.
"""
import functools
import http.server
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time
import sys
import urllib.request

if "--public-probe" in sys.argv and "--chain" in sys.argv:
    raise SystemExit("Run --public-probe and --chain separately: each needs its own browser state.")

ROOT = Path(__file__).resolve().parents[1]
PASSWORD = "URMA public test password 2026"
VAULT = list((ROOT / "app-engine/tests/fixtures/valid.vault").read_bytes())


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


DIST = Path(os.environ.get("WIRE_DIST", ROOT / "dist"))
server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(QuietHandler, directory=str(DIST)))
threading.Thread(target=server.serve_forever, daemon=True).start()
origin = f"http://127.0.0.1:{server.server_port}"
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    driver_port = sock.getsockname()[1]
driver_url = f"http://127.0.0.1:{driver_port}"


def request(path, value=None, method=None):
    data = None if value is None else json.dumps(value).encode()
    req = urllib.request.Request(driver_url + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as response:
        result = json.load(response)["value"]
    if isinstance(result, dict) and "error" in result and "stacktrace" in result:
        raise AssertionError(result)
    return result


with tempfile.TemporaryDirectory(prefix="wire-fixture-", dir=ROOT / "target") as profile:
    driver = subprocess.Popen(["chromedriver", f"--port={driver_port}"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    session = None
    try:
        for _ in range(100):
            try:
                request("/status")
                break
            except OSError:
                time.sleep(.1)
        session = request("/session", {"capabilities": {"alwaysMatch": {"browserName": "chrome",
            "goog:chromeOptions": {"binary": "/usr/bin/chromium", "args": ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", f"--user-data-dir={profile}"]}}}})["sessionId"]
        base = "/session/" + session
        request(base + "/timeouts", {"script": 150000, "pageLoad": 30000})
        request(base + "/goog/cdp/execute", {"cmd": "Page.addScriptToEvaluateOnNewDocument", "params": {"source": "const fixtureFetch=window.fetch; window.fixtureNetworkFetch=fixtureFetch; window.fetch=(input,options)=>{ const url=new URL(typeof input==='string'?input:input.url,location.href); if(url.origin!==location.origin)return Promise.reject(new Error('Fixture prohibits live chain requests')); return fixtureFetch(input,options); };"}})

        def js(script, *args):
            return request(base + "/execute/sync", {"script": script, "args": list(args)})

        def wait_loaded():
            for _ in range(200):
                if js("return !!window.wasmBindings?.browser_fixture_command && !!navigator.serviceWorker.controller"):
                    return
                time.sleep(.1)
            raise AssertionError("Application/worker did not load")

        def engine_request(cmd, args):
            chain = "litecoin"
            requests = {
                "identity_status": lambda: "IdentityStatus",
                "import_identity": lambda: {"ImportIdentity": {"vault": args["vaultBytes"], "password": args["password"]}},
                "unlock_wallet": lambda: {"Unlock": {"Password": args["password"]}},
                "lock_wallet": lambda: "Lock",
                "set_chain_source": lambda: {"AdoptSource": {"url": args["url"]}},
                "get_wallet_summary": lambda: {"Wallet": {"chain": chain}},
                "get_feed": lambda: {"Feed": {"chain": chain}},
                "quote_publication": lambda: {"Quote": {"chain": chain, "content": {"Post": {
                    "text": args["text"], "wire": args.get("wire", ""),
                    "hashtags": args.get("hashtags", []), "addressed": "Standalone"}}}},
                "advance_publication": lambda: {"Advance": {"id": args["id"], "approve": args["approve"]}},
                "publication_status": lambda: "PublicationStatus",
            }
            return requests[cmd]()

        def engine_response(answer):
            if isinstance(answer, str):
                return answer
            (variant, value), = answer.items()
            if variant == "Identity":
                value["unlocked"] = value["session"] != "Locked"
            if variant == "Pending":
                return None if value == "Settled" else value["Awaiting"]
            return value

        def command(cmd, args=None, failure=False):
            args = dict(args or {})
            fixture = cmd.startswith("fixture-")
            if cmd == "fixture-block":
                args.setdefault("nonce", 201)
            name, payload = (cmd, args) if fixture else ("engine", engine_request(cmd, args))
            result = request(base + "/execute/async", {"script": "const done=arguments[2]; window.wasmBindings.browser_fixture_command(arguments[0],JSON.stringify(arguments[1])).then(value=>done({value}),error=>done({error:String(error)}));", "args": [name, payload]})
            assert ("error" in result) == failure, result
            if failure:
                return result["error"]
            if cmd == "fixture-sign":
                return result["value"]
            answer = json.loads(result["value"])
            return answer if fixture else engine_response(answer)

        request(base + "/url", {"url": origin})
        wait_loaded()
        if "--public-probe" in sys.argv:
            js("window.blockedFixtureFetch=window.fetch;window.fetch=window.fixtureNetworkFetch")
            probe_url = next((arg.split("=", 1)[1] for arg in sys.argv if arg.startswith("--public-source=")), "https://litecoinspace.org/testnet/api")
            probe = command("fixture-source-probe", {"url": probe_url})
            print("PASS: read-only direct browser public genesis/tip/block integrity probe", probe)
            js("window.fetch=window.blockedFixtureFetch")
        assert command("identity_status")["source"] == "missing"
        command("import_identity", {"vaultBytes": VAULT, "password": "wrong"}, failure=True)
        assert not command("identity_status")["vault_available"]
        imported = command("import_identity", {"vaultBytes": VAULT, "password": PASSWORD})
        assert imported["unlocked"] and imported["source"] == "imported"
        signature = command("fixture-sign")
        assert len(signature) == 128
        command("import_identity", {"vaultBytes": VAULT, "password": PASSWORD}, failure=True)
        command("lock_wallet")
        command("fixture-sign", failure=True)
        assert not command("identity_status")["unlocked"]
        command("unlock_wallet", {"password": PASSWORD, "useUnlockFile": False})
        assert command("fixture-sign") == signature
        if "--chain" in sys.argv:
            from browser_chain_fixture import run_chain_fixture
            run_chain_fixture(command, js, request, base)
        # Stop the actual origin server; do not toggle the browser offline flag.
        server.shutdown()
        server.server_close()
        request(base + "/url", {"url": origin + "/settings"})
        wait_loaded()
        locked = command("identity_status")
        assert locked["vault_available"] and not locked["unlocked"]
        if "--chain" in sys.argv:
            cached = command("get_feed", {"chain": "ltc"})
            assert len(cached["posts"]) == 1 and not cached["complete_to_tip"]
            saved = command("publication_status")
            assert saved["approved"] and "unknown" in saved["message"] and not saved["complete"]
        command("fixture-sign", failure=True)
        command("unlock_wallet", {"password": PASSWORD, "useUnlockFile": False})
        assert command("fixture-sign") == signature
        # The only identity record must be the original ciphertext byte array.
        persisted = request(base + "/execute/async", {"script": "const done=arguments[0]; const r=indexedDB.open('urma-wire',1); r.onsuccess=()=>{const db=r.result; const q=db.transaction('private').objectStore('private').getAll();q.onsuccess=()=>{done(q.result);db.close();};};", "args": []})
        assert json.dumps(VAULT, separators=(",", ":")) in persisted
        assert all(PASSWORD not in value for value in persisted)
        assert PASSWORD not in js("return JSON.stringify(localStorage)+JSON.stringify(sessionStorage)")
        print("PASS: real WASM vault import/unlock/sign/lock; rejected wrong password/overwrite; origin stopped deep-link reload; ciphertext durable; session locked after reload; no password persisted")
    finally:
        if session:
            request("/session/" + session, method="DELETE")
        driver.terminate()
        driver.wait(timeout=10)
        server.shutdown()
        server.server_close()
