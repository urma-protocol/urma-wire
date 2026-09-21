// Deterministic atomic-upgrade checks; no network, browser profile or private data.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const {webcrypto, createHash} = require('node:crypto');
const stores = new Map();
const caches = {
    async open(name) {
        if (!stores.has(name)) stores.set(name, new Map());
        const data = stores.get(name);
        return {async put(key, value) { data.set(key, value); }, async match(key) { return data.get(key); }};
    },
    async delete(name) { return stores.delete(name); },
};
function worker(version, corrupt = false) {
    const events = {};
    const body = 'complete application';
    const context = {
        VERSION: version,
        ASSETS: [{url: '/index.html', sha256: createHash('sha256').update(body).digest('hex')}],
        caches, crypto: webcrypto, Response, URL,
        fetch: async () => new Response(corrupt ? 'incomplete upgrade' : body),
        self: {location: {origin: 'https://fixture.invalid'}, clients: {claim: async () => {}},
            addEventListener(name, callback) { events[name] = callback; }},
    };
    vm.runInNewContext(fs.readFileSync('web/sw.js', 'utf8'), context);
    return events;
}
async function install(events) {
    let task;
    events.install({waitUntil(value) { task = value; }});
    return task;
}
(async () => {
    await install(worker('previous'));
    assert(stores.get('urma-wire-shell-previous').has('/__urma_complete__'));
    await assert.rejects(install(worker('upgrade', true)), /changed during download/);
    assert(!stores.has('urma-wire-shell-upgrade'));
    assert(stores.get('urma-wire-shell-previous').has('/__urma_complete__'));
    await install(worker('upgrade'));
    assert(stores.get('urma-wire-shell-upgrade').has('/__urma_complete__'));
    assert(stores.get('urma-wire-shell-previous').has('/__urma_complete__'));
    let intercepted = false;
    worker('upgrade').fetch({request: {url: 'https://fixture.invalid/private-vault', method: 'GET'}, respondWith() { intercepted = true; }});
    assert(!intercepted, 'Non-manifest private resources must not enter asset caching');
    console.log('PASS: complete cache retained on corrupt update; candidate removed; successful update complete; private URL excluded');
})().catch(error => { console.error(error); process.exitCode = 1; });
