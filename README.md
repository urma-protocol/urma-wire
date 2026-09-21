# URMA Wire V0

Tauri 2 desktop shell with a Leptos/WASM frontend. The current end-to-end
milestone is **public text posts and replies on Litecoin Testnet**. URMA
manifest dependencies use crates.io version 0.2.0, including the structured
wire/hashtag extension. No local override is required.

## Run locally

With Rust's `wasm32-unknown-unknown` target, Trunk, Cargo Tauri and the platform's
Tauri native dependencies already installed:

```sh
NO_COLOR=true cargo tauri dev
```

The running app must be restarted by its operator to load new native code.
For development against unpublished library changes only,
`cargo --config .cargo/local-urma.toml ...` opts into the neighboring `../urma`
workspace. For Tauri/Trunk subprocesses, merge that patch table into an ignored
`.cargo/config.toml`; do not overwrite an existing config. No override is enabled
automatically or needed for this release.

For frontend-only builds: `trunk build --release`. If the surrounding shell sets
`NO_COLOR=1`, use `NO_COLOR=true trunk build --release` (Trunk expects a boolean).
The browser build supports local encrypted identity import, session unlock and
explicit Lock using the same published URMA vault library as native Wire.
Verified chain sync, balances and publication still require the native shell;
browser identity access is not a complete browser publisher. No node or miner is
started by the app.

## Browser offline identity access

Serve the complete `trunk build --release` output at an origin's root over HTTPS
(localhost is accepted for development). Python 3 is required by the Trunk
post-build manifest hook. It includes every emitted file, including hashed WASM,
JavaScript snippets, CSS and images. The service worker verifies every file's
SHA-256 before marking its version complete. Only then does the application show
its offline-ready identity message. Interrupted or changed downloads cannot
replace the previous complete version. Updates activate when older tabs close;
previous complete caches are retained. Deep-link reload uses the cached shell
even if the hosting server is unavailable. There are no remote startup fonts.

The imported URMA vault bytes persist in IndexedDB, separate from public asset
caches. Import verifies the password and refuses to overwrite an existing vault,
including concurrent imports from different tabs. Keys are held only in WASM
session memory and dropped on Lock/reload; neither the service worker nor browser
storage holds the password or unlocked keys. Each tab has its own locked session.
The storage button requests persistent storage and reports a denial honestly.
Keep the original encrypted vault backup: browser data can still be deleted by
the user, browser policy or storage eviction. No persistent auto-unlock exists.

Browser chain commands fail explicitly; they never synthesize a balance, bypass
proof checks, or send keys to a server. Completing browser publication needs an
async chain-source interface using browser fetch to selectable CORS-enabled
sources, genesis/record/author proof verification, an IndexedDB-backed verified
index with rollback checkpoints, and transactional saved-plan/approval/journal
storage plus cross-tab funding exclusion. Current native runtime operations use
synchronous RPC and filesystem persistence. Those must be adapted while retaining
exact fee approval and commit-before-reveal confirmation; no backend proxy or
custodial service is implied by this browser identity foundation.

Focused nonvisual browser fixture check, using installed Chromium/chromedriver
and a disposable profile (no installation, live chain source or user vault):

```sh
NO_COLOR=true trunk build --features browser-fixture-test --offline
python3 scripts/browser_fixture_test.py
```

The optional fixture feature exports a local test harness; omit it from normal
builds. It checks the committed public vault, password failure, signature
verification, explicit Lock, duplicate import, actual origin-server shutdown and
deep-link reload, retained ciphertext and locked reload. It takes no screenshots.

## Branding

`assets/urma-wire.svg` contains the existing URMA Wire mark with its ROSINT eagle
embedded, so it has no external asset dependency. The header and browser icon
use this animated SVG (with its reduced-motion variant). The static
`assets/urma-wire-icon.png` supplies the Tauri desktop/mobile icons. Regenerate
those sizes with `cargo tauri icon assets/urma-wire-icon.png`.

## Testnet flow

1. Identity → explicitly import an encrypted URMA vault or unlock the configured
   active URMA identity with its vault password. Desktop also offers **Use
   configured unlock file** when present. No identity is created. Wallet shows
   network funding separately from the identity session.
2. Write a post/reply → **Review & publish** prepares and saves signed bytes,
   without broadcast. Optional wire and hashtag inputs are structured protocol
   fields. The review displays the exact frozen text, wire, hashtags, reply target,
   total fee, selected funding and value returned. Fee rate is 1 litoshi/vbyte
   for this milestone; there is no automatic fee increase. Funding currently
   needs one adequate confirmed spendable output; total balance is insufficient
   to prove that such an output exists.
3. **Approve fee & publish** approves that exact plan and begins submission.
   The commit must confirm before its reveal. The frontend reconciles every
   two seconds and reports both transactions; success requires both confirmed.
   A source error or rejection stops following with its reason. Resolve it and
   choose **Resume publication**; no generic infinite retry or fee bump occurs.
4. Completion refreshes the confirmed feed. **Refresh confirmed feed** also
   advances the index explicitly. No mempool record is inserted into the feed.

Pause stops after the in-flight request. Closing Wire stops following; reopen
and resume the saved signed plan without unlocking again. Locking the identity
prevents new signing but does not revoke an already approved signed plan.
The app admits one unfinished approved publication and one instance per data
directory, preventing concurrent app plans from reusing funding.

## Configuration and state

Reads `URMA_CONFIG`, or `~/.config/urma/config.json`: `rpc_url`, `node_auth_file`,
`vault`, `unlock_file`. RPC/auth environment overrides are `URMA_RPC_URL` and
`URMA_NODE_AUTH_FILE`. Both RPC fields must be supplied together; otherwise the
URMA public Litecoin Testnet source pool is used. Configured RPC failure never
silently redirects credentials to a public endpoint. Genesis is checked.
Settings are loaded at startup; restart after changing configuration.

Default vault/unlock paths match the CLI. Passwords are never saved by Wire.
The Identity page copies a validated encrypted vault into app-data `identity.urma`,
with the URMA 0.2 limit of 4532 bytes. Import requires its password, validates its
active author before writing, and never overwrites an existing imported vault or
the CLI vault. The imported copy takes precedence for future unlocks. Its keys
stay in the running session until Lock or process exit; reloading the frontend
only reads public session status and never automatically unlocks. A source or
balance failure does not lock the identity or change its author.
State is under Tauri's platform app-data directory for `org.urma.wire`:
`plans/<plan-id>/plan.json`, exact approval, progress journal, `active.json`,
`ltc-testnet-index.json` and `ltc-testnet-start.json`. Existing prototype
`wire_store.json` is left untouched and not used as verified chain evidence.

First sync starts 100 blocks before the current tip and remembers that height.
Each refresh processes at most 100 blocks, with the indexed range and remaining
catch-up shown explicitly. This is a recent-history feed, not full-chain search.
The library verifies record/author proofs and rolls back orphaned index entries.
Posts and replies are rendered as inert text. Profile editing, avatar publication,
Markdown/external media fetching and Bitcoin UI integration remain separate work.

## Wires, hashtags and local follows

A wire such as `/rosint` is an open channel, with no owner or registration;
using a label never proves affiliation. Each post/reply can name one wire and
up to eight hashtags such as `#investigation`, shared across wires on the same
network. Leaving both fields empty keeps legacy kind 02/04 encoding. Structured
records use new kinds 0A/0B. Names use 1–32 lowercase ASCII letters/digits, `-`
or `_`, beginning with a letter/digit; the UI lowercases label input and sorts
and deduplicates hashtags before exact approval. Text remains unchanged.

Global feed includes all records and clears both optional filters. Separate wire
and hashtag controls work independently: either alone filters that dimension;
both together require a record to match both (AND). Selecting or clearing one
does not change the other. Each Follow button subscribes only to its individual
wire or hashtag, never a combined filter. Following remains a separate view of
the union of locally followed labels (OR). Filters apply before taking the
latest 100 results. Follows persist in WebView localStorage, keyed by chain and
network name, with a local limit of 128; no transaction or payment is involved.
They are tied to this local WebView profile. Storage failures are shown.
The initial network options are fixed and only Litecoin Testnet is enabled.

Index format 2 rescans a format-1 cache from its recorded starting height, because
old readers could have skipped the newly allocated kinds. This occurs on the
next explicit sync, uses the normal bounded slices and retains chain validation.
No full-chain history or reduced blockchain scanning cost is implied.

## Focused local checks

Coin tab logos are bundled locally: Litecoin from
https://github.com/spothq/cryptocurrency-icons/blob/master/svg/color/ltc.svg
(CC0), and Bitcoin from
https://github.com/bitpay/bitcoin-brand/blob/master/bitcoin.svg.

The UI uses Thaw 0.4 with Leptos 0.7. The network selector uses Thaw Menu;
the four choices remain visible while only Litecoin Testnet is enabled.

The sidebar logo plays once on load, then once every five minutes. Future UI
events can replay it with `window.dispatchEvent(new Event("urma:logo-animate"))`.
Each replay uses the existing 3.8-second trace animation and rests on the repaired
green mark. Reduced-motion preferences disable the animation. The timer and
event listener are removed when the navigation component unmounts.

```sh
cargo test -p wire-app-shared --locked
cargo check --workspace --locked
cargo check -p wire-app-ui --target wasm32-unknown-unknown --locked
```

Tests use temporary files, public fixture identities and a loopback RPC mock;
they do not start a blockchain node or broadcast to a live network. They cover
exact approval, post/reply preparation, confirmation stages, read-only status,
restart, unchanged retransmission, rejection/source failure and feed projection.
They do not establish live acceptance of this desktop application.
