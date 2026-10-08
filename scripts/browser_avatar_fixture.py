"""Legacy signed-avatar refusal in the existing disposable browser fixture gate."""
import json
import time


def run_legacy_avatar_fixture(command, js, request, base, fixture_path):
    fixture = json.loads(fixture_path.read_text())
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
    stored(active)

    def refused():
        for cmd, args in [
            ("publication_status", {}),
            ("advance_publication", {"id": fixture["id"], "approve": False}),
            ("advance_publication", {"id": fixture["id"], "approve": True}),
            ("discard_quote", {"id": fixture["id"]}),
        ]:
            error = command(cmd, args, failure=True)
            assert "exactly 128" in error, error
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
    print("PASS: real IndexedDB signed legacy avatar status/resume/approve/discard refusal; exactly-128 error; no mocked submissions; saved active bytes unchanged across locked restart")
