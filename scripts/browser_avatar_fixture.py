"""Legacy signed-record refusal in the existing disposable browser fixture gate."""
import json
import time


def profile_editor_fixture(js):
    js("history.pushState({},'', '/identity');window.dispatchEvent(new PopStateEvent('popstate'))")
    for _ in range(100):
        if js("return !!document.querySelector('textarea')"):
            break
        time.sleep(.05)
    else:
        raise AssertionError("Profile editor did not mount")
    js("[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Refresh session status').click()")

    def snapshot():
        return js("""
            const t=document.querySelector('textarea');
            const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()==='Review nickname & fee');
            return {text:t.value,counter:t.closest('label').querySelector('.field-hint').textContent,
                warning:t.closest('label').querySelector('[role=alert]').textContent,
                disabled:b.disabled,maxlength:t.hasAttribute('maxlength')};
        """)

    for _ in range(100):
        if not snapshot()["disabled"]:
            break
        time.sleep(.05)
    else:
        raise AssertionError("Unlocked profile editor review remained disabled")
    for text in ["x" * 128, "x" * 129, "😀" * 32, "😀" * 33, "😀" * 32]:
        count = len(text.encode("utf-8"))
        js("const t=document.querySelector('textarea');t.value=arguments[0];t.dispatchEvent(new Event('input',{bubbles:true}))", text)
        for _ in range(100):
            state = snapshot()
            if (f"{count} / 128 UTF-8 bytes." in state["counter"]
                    and state["disabled"] == (count > 128)):
                break
            time.sleep(.02)
        else:
            raise AssertionError(state)
        assert state["text"] == text, state
        assert not state["maxlength"], state
        assert ("Nickname exceeds" in state["warning"]) == (count > 128), state
    print("PASS: real browser nickname textarea/counter/review enabled128-disabled129; emoji32=128/33=132/re-enabled128; no maxlength or truncation")


def run_legacy_avatar_fixture(command, js, request, base, fixture_path):
    fixture = json.loads(fixture_path.read_text())
    error_contains = fixture.get("error_contains", "exactly 128")
    kind = fixture.get("kind", "avatar")
    active = json.dumps({
        "plan": fixture["plan"], "approved": fixture["id"],
        "attempted": [], "observations": [], "last_error": None,
    }, separators=(",", ":"))
    key = "ltc-testnet-active-plan"

    def stored(value=None):
        return request(base + "/execute/async", {
            "script": """
                const [key,value,done]=arguments, r=indexedDB.open('urma-wire',1);
                r.onsuccess=()=>{
                    const db=r.result, tx=db.transaction('private',value===null?'readonly':'readwrite');
                    const store=tx.objectStore('private');
                    const q=value===null?store.get(key):store.put(value,key);
                    tx.oncomplete=()=>{done(q.result);db.close();};
                    tx.onerror=()=>done({error:String(tx.error)});
                };
                r.onerror=()=>done({error:String(r.error)});
            """, "args": [key, value],
        })

    install = """
        window.avatarFixtureSubmissions=[];
        const priorFetch=window.fetch;
        window.fetch=async(input,options={})=>{
            const url=new URL(input instanceof Request?input.url:String(input),location.href);
            const method=(options.method??input.method??'GET').toUpperCase();
            if(url.origin===location.origin)return priorFetch(input,options);
            if(method==='POST') {
                avatarFixtureSubmissions.push({url:url.href,body:options.body??input.body});
                throw new Error('Legacy fixture forbids submission');
            }
            if(url.origin!=='https://fixture.invalid')throw new Error('Legacy fixture forbids external origin: '+url.origin);
            if(url.pathname==='/api/block-height/0')return new Response(
                '4966625a4b2851d9fdee139e56211a0d88575f59ed816ff5e6a63deb4e3e29a0');
            throw new Error('Legacy fixture has no chain reads: '+url.pathname);
        };
    """
    js(install)
    command("set_chain_source", {"url": "https://fixture.invalid/api"})
    if kind == "profile":
        profile_editor_fixture(js)
    stored(active)

    def refused():
        for cmd, args in [
            ("publication_status", {}),
            ("advance_publication", {"id": fixture["id"], "approve": False}),
            ("advance_publication", {"id": fixture["id"], "approve": True}),
            ("discard_quote", {"id": fixture["id"]}),
        ]:
            error = command(cmd, args, failure=True)
            assert error_contains in error, error
            assert "Keep a backup" in error, error
            assert "Do not delete the saved publication" in error, error
            assert stored() == active
        assert js("return avatarFixtureSubmissions.length") == 0

    refused()
    request(base + "/url", {"url": js("return location.href")})
    for _ in range(200):
        if js("return !!window.wasmBindings?.browser_fixture_command"):
            break
        time.sleep(.1)
    else:
        raise AssertionError("Browser fixture failed to reload")
    js(install)
    assert not command("identity_status")["unlocked"]
    assert stored() == active
    refused()
    print(f"PASS: real IndexedDB signed legacy {kind} status/resume/approve/discard refusal; {error_contains} error; no mocked submissions; saved active bytes unchanged across locked restart")
