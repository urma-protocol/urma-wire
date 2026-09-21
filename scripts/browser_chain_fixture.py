"""Synthetic Esplora fixture; every request stays inside browser memory."""
import json
import time


def run_chain_fixture(command, js, request, base):
    fixture = command("fixture-chain")
    js("window.chainFixture=arguments[0]", fixture)
    install = r"""
        const f=window.chainFixture;
        if(!f.statuses) {
            f.tip=200; f.submissions=[]; f.raws={[f.funding.txid]:f.funding.raw};
            f.statuses={[f.funding.txid]:{confirmed:true,block_height:1,block_hash:f.blocks[0].hash}};
        }
        const fallback=window.fetch;
        window.fetch=async (input,options={})=>{
            const u=new URL(input,location.href);
            if(u.origin!=='https://fixture.invalid')return fallback(input,options);
            if(f.unavailable)throw new Error('fixture source stopped');
            const p=u.pathname.replace('/api','');
            const respond=x=>new Response(typeof x==='string'?x:JSON.stringify(x));
            if(p==='/block-height/0')return respond(f.wrongGenesis?'00'.repeat(32):f.genesis);
            if(p==='/blocks/tip/height')return respond(String(f.tip));
            if(p.startsWith('/block-height/'))return respond(f.blocks[Number(p.split('/').pop())-1].hash);
            if(p.startsWith('/block/')&&p.endsWith('/raw')){
                const block=f.blocks.find(b=>b.hash===p.split('/')[2]);
                if(!block)return new Response('',{status:404});
                let raw=block.raw;
                if(f.corruptHash===block.hash)raw=raw.slice(0,-2)+(raw.endsWith('00')?'01':'00');
                return new Response(Uint8Array.from(raw.match(/../g),b=>parseInt(b,16)));
            }
            if(p.startsWith('/address/'))return respond([{txid:f.funding.txid,vout:0,value:f.funding.value,status:f.statuses[f.funding.txid]}]);
            if(p==='/tx'&&options.method==='POST'){
                if(f.reject) return new Response('fixture policy rejected',{status:400});
                const id=JSON.parse(await window.wasmBindings.browser_fixture_command('fixture-txid',JSON.stringify({raw:options.body})));
                f.submissions.push(options.body); f.raws[id]=options.body; f.statuses[id]={confirmed:false};
                if(f.loseReply) { f.loseReply=false; throw new Error('fixture response lost after acceptance'); }
                return respond(id);
            }
            const id=p.split('/')[2];
            if(p.endsWith('/hex'))return f.raws[id]?respond(f.raws[id]):new Response('',{status:404});
            if(p.endsWith('/status'))return f.statuses[id]?respond(f.statuses[id]):new Response('',{status:404});
            if(p.includes('/outspend/'))return respond({spent:false});
            throw new Error('Unexpected fixture route '+p);
        };
    """
    js(install)
    command("set_chain_source", {"url": "https://fixture.invalid/api"})
    js("chainFixture.wrongGenesis=true")
    command("set_chain_source", {"url": "https://fixture.invalid/api"}, failure=True)
    js("chainFixture.wrongGenesis=false")
    wallet = command("get_wallet_summary", {"chain": "ltc"})
    assert wallet["confirmed_balance_sats"] == 100_000_000
    first = command("get_feed", {"chain": "ltc"})
    assert first["start"] == 100 and first["tip"] == 199 and not first["complete_to_tip"], first
    second = command("get_feed", {"chain": "ltc"})
    assert second["tip"] == 200 and second["complete_to_tip"]
    args = {"chain": "ltc", "text": "browser fixture post", "wire": "rosint", "hashtags": ["investigation"]}
    quote = command("quote_publication", args)
    assert not quote["approved"] and quote["fee"] == quote["selected_funding"]-quote["returned"]
    assert js("return chainFixture.submissions.length") == 0
    command("advance_publication", {"id": "wrong", "approve": True}, failure=True)
    command("advance_publication", {"id": quote["id"], "approve": False}, failure=True)
    js("chainFixture.loseReply=true")
    command("advance_publication", {"id": quote["id"], "approve": True}, failure=True)
    assert js("return chainFixture.submissions.length") == 1
    command("quote_publication", args, failure=True)
    command("lock_wallet")
    # Restart the page with durable approval, no session keys and the exact prior bytes.
    network_state = js("return chainFixture")
    request(base + "/url", {"url": js("return location.href")})
    for _ in range(200):
        if js("return !!window.wasmBindings?.browser_fixture_command"):
            break
        time.sleep(.1)
    js("window.chainFixture=arguments[0]", network_state)
    js(install)
    assert not command("identity_status")["unlocked"]
    resumed = command("advance_publication", {"id": quote["id"], "approve": False})
    assert resumed["mempool"] == 1 and not resumed["complete"]
    assert js("return chainFixture.submissions.length") == 1

    def confirm():
        pending = js("return Object.entries(chainFixture.statuses).filter(([id,s])=>!s.confirmed).map(([id])=>({id,raw:chainFixture.raws[id]}))")
        previous = js("return chainFixture.blocks[chainFixture.tip-1].hash")
        block = command("fixture-block", {"previous": previous, "transactions": [item["raw"] for item in pending]})
        js("const b=arguments[0]; chainFixture.blocks.push(b);chainFixture.tip++; for(const id of arguments[1])chainFixture.statuses[id]={confirmed:true,block_height:chainFixture.tip,block_hash:b.hash}", block, [item["id"] for item in pending])
        return block

    confirm()
    advanced = command("advance_publication", {"id": quote["id"], "approve": False})
    assert advanced["confirmed"] == 1 and advanced["mempool"] == 1
    assert js("return chainFixture.submissions.length") == 2
    reveal_block = confirm()
    complete = command("advance_publication", {"id": quote["id"], "approve": False})
    assert complete["complete"] and complete["confirmed"] == 2
    feed = command("get_feed", {"chain": "ltc"})
    assert len(feed["posts"]) == 1 and feed["posts"][0]["text"] == args["text"]
    assert feed["posts"][0]["wire"] == "rosint"
    # Replace the last block at the same height: cached hash rollback removes its post.
    previous = js("return chainFixture.blocks[chainFixture.tip-2].hash")
    replacement = command("fixture-block", {"previous": previous, "transactions": [], "nonce": 999})
    js("chainFixture.blocks[chainFixture.tip-1]=arguments[0]", replacement)
    rolled = command("get_feed", {"chain": "ltc"})
    assert rolled["posts"] == []
    # Corrupt a new block body: no checkpoint is advanced, cached verified state survives.
    corrupt = command("fixture-block", {"previous": replacement["hash"], "transactions": [], "nonce": 1000})
    js("chainFixture.blocks.push(arguments[0]);chainFixture.tip++;chainFixture.corruptHash=arguments[0].hash", corrupt)
    failed = command("get_feed", {"chain": "ltc"})
    assert failed["tip"] == rolled["tip"] and not failed["complete_to_tip"]
    # Restore the real reveal block for the final offline cached-history reload.
    js("chainFixture.tip=202;chainFixture.blocks.length=202;chainFixture.blocks[201]=arguments[0];chainFixture.corruptHash=null", reveal_block)
    assert len(command("get_feed", {"chain": "ltc"})["posts"]) == 1
    # Origin-wide lock rejects overlapping operations without selecting funding twice.
    request(base + "/execute/async", {"script": "const done=arguments[0];navigator.locks.request('urma-wire-chain-operation',()=>new Promise(resolve=>{window.releaseFixtureLock=resolve;done(true)}))", "args": []})
    command("quote_publication", args, failure=True)
    js("releaseFixtureLock()")
    command("unlock_wallet", {"password": "URMA public test password 2026", "useUnlockFile": False})
    print("PASS: mocked browser balances; bounded checkpoint sync; exact quote/approval; lost POST response reconciliation; locked resume; commit-before-reveal; verified feed; reorg rollback; corrupt block rejection; origin-wide operation lock")
