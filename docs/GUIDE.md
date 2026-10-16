# URMA Wire guide

[Project overview and build instructions](../README.md).

Commands below run from the application repository root. This guide covers the
experimental desktop and browser implementations, saved state and focused tests.

## Automatic local block source

An installed URMA Node takes precedence over everything below: see
[automatic sources](../README.md#automatic-sources). Without one, in native
**Public sources** mode, Wire checks the loopback RPC daemon in the background
about every three seconds: `rpc_url` and `node_auth_file` from the URMA config,
or `http://127.0.0.1:19332` with `~/.litecoin/testnet4/.cookie`. The daemon may
start later: no app restart or source-selection change is needed. Each check reads the
current cookie, verifies the Litecoin Testnet genesis and requires initial block
download to be finished with blocks caught up to the node's headers.

Once verified, the local daemon is preferred for block hashes, headers and raw
blocks through the existing router, including an already-running index scan.
An in-flight public request is allowed to finish or time out; subsequent requests
can use the local source. Missing/pruned blocks, RPC failure or invalid responses
fall back through the normal public-source validation. Availability is probed
again after loss or cookie rotation. RPC operations retain their existing bounded
network timeouts, so a hung daemon can take longer than the polling interval.

The monitor icon labelled **Local**, beneath the loading counter, is separate
from P2P peers and remote RPC/API counts. Its tooltip describes availability;
ready does not certify that a particular block was served locally. Public tip
selection, transaction lookup and publication/submission routing remain as before:
this automatic addition is block-read-only and never changes `source.json`.
Explicit **Local node → Verify & use** still selects local-only operations.
The detector does not start the system service, create credentials, scan ports,
or reload edited RPC configuration while running. Browser/WASM builds cannot
probe the native daemon.

## Desktop frontend boundary

The packaged desktop frontend has an explicit Content Security Policy in
`src-tauri/tauri.conf.json`: local scripts/assets and Tauri IPC, inert data images,
no frames, objects or workers. Inline styles remain allowed for Thaw/Leptos;
WebAssembly compilation is explicitly allowed. Native chain and checked image
requests go through the engine, not arbitrary page JavaScript connections.
The `main` window capability remains local and the host exposes the typed
`engine` command. This configuration is a boundary, not an exhaustive security
audit or a claim that all requested engine operations need no validation.

`trunk serve` supplies its own development CSP header from `Trunk.toml`, including
a per-response script nonce, the local reload WebSocket, and HTTPS for the
frontend-only browser engine. Its policy is intentionally different from the
packaged native frontend. Development mode does not install the offline service
worker. A separately hosted release browser build still needs a hosting-policy
review; Tauri's native CSP is not automatically a server policy for that build.

## Browser offline operation

Historical evidence recorded in README revision
`ef850ea09c6d50d352d35c4902e937c67424cafe` (not a claim of current verification;
the execution date and tested source revision are not recorded here): the browser
engine passed synthetic-chain publication,
restart and offline tests. A direct browser read of Litecoin Testnet block
4892707 passed transparent transaction Merkle and witness commitment verification
with the shared Litecoin decoder. Esplora's transparent block encoding is handled
explicitly, separately from full node serialization. The result is a transparent
read projection; it is not a full MWEB consensus verifier. Confidential MWEB
transactions remain unsupported, and extension contents are not authenticated by
this reader. This read-only check is not evidence of a live browser publication.
These observations concern the locally integrated implementation; the committed
patches select the sibling core checkout rather than establish published-package
equivalence.

Serve the complete `trunk build --release` output at an origin's root over HTTPS
(localhost is accepted for development). Python 3 is required by the Trunk
post-build manifest hook. It includes every emitted file, including hashed WASM,
JavaScript snippets, CSS and images. The service worker verifies every file's
SHA-256 before marking its version complete. Only then does the application show
its offline-ready message. Interrupted or changed downloads cannot
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

Settings selects a public Litecoin Testnet Esplora API URL. Sources must support
HTTPS and CORS. See the [configuration sources](#configuration-and-state) for
endpoint defaults.
Wire checks genesis before using a source and never silently replaces a configured
source. Fetch uses no cookies or backend proxy. Unreachable sources, CORS denial,
rejections and malformed proofs produce explicit errors. Browser privacy policy
and endpoint availability can still prevent a connection.

IndexedDB stores the recent-history starting height, contiguous block-hash
checkpoints and verified records. A sync run resumes from the saved checkpoint,
rolls back changed hashes, verifies blocks in order up to the tip and saves the
index per batch. Transparent block Merkle roots, witness commitments and URMA
author proofs are checked with the shared library. Cached history remains
readable offline with a stale-source notice; it is not marked caught up. This
uses public-provider chain observations, not an independently validating node or
full-chain history. No offline transaction confirmation is claimed.

Funding checks exact raw transaction amounts/scripts, block inclusion, unspent
status and coinbase maturity. The account's addresses are listed with a gap
limit of 20 per chain; only the 64 largest confirmed coins are verified and
offered for funding, so dust sent to the public address cannot block publishing. Preparing stores an immutable signed plan with an
exact quoted fee, without submitting. Approval saves that plan's validated ID
before any broadcast. The journal persists submission intent and observations;
restart or a lost response reconciles the same signed bytes. Reveal submission
follows as soon as the commit is observed in the mempool or confirmed. Success
still requires both transactions to confirm. The public Esplora path has no
mempool preflight; policy rejection pauses publication without fee changes. Approved plans resume
while the vault is locked. Offline plan views show saved bytes and explicitly
unknown current confirmation status. Origin-wide Web Locks serialize chain
operations across tabs, and an unfinished approved plan blocks new preparation.
Browsers without Web Locks fail safely for these operations.

Focused nonvisual browser fixture check, using installed Chromium/chromedriver
and a disposable profile (no installation, live chain source or user vault):

```sh
NO_COLOR=true trunk build --features fixture-harness --offline
python3 scripts/browser_fixture_test.py --chain
node scripts/offline_worker_test.js
```

The optional fixture feature exports a local test harness; omit it from normal
builds. It checks the committed public vault, password failure, signature
verification, explicit Lock, duplicate import, actual origin-server shutdown and
deep-link reload, retained ciphertext and locked reload. Mocked Esplora checks
cover balances, bounded sync, exact approval, accepted-but-lost broadcast replies,
restart with a locked vault, commit-before-reveal, reorg rollback, corrupt block
rejection, operation locking, and offline cached history/plans. No live broadcast
is used. Optional `--public-probe` only fetches a public genesis, tip and block to
check actual browser CORS and block decoding. Run it separately from `--chain`,
so real-provider observations never mix with synthetic browser state. These
checks take no screenshots.

## Branding

`assets/urma-wire.svg` contains the existing URMA Wire mark with its ROSINT eagle
embedded, so it has no external asset dependency. The header and browser icon
use this animated SVG (with its reduced-motion variant). The static
`assets/urma-wire-icon.png` supplies the Tauri desktop/mobile icons. Regenerate
those sizes with `cargo tauri icon assets/urma-wire-icon.png`.

## Testnet flow

0. Chain source → Settings shows the selected source and the providers it
   holds; **Public sources** is the default and works without a node. The
   Connection page reports the source height, the locally indexed range and
   the evidence tier of the provider that answered; see
   [configuration and state](#configuration-and-state) for the tiers.
1. Identity → explicitly import an encrypted URMA vault or unlock the configured
   active URMA identity with its vault password. Desktop also offers **Use
   configured unlock file** when present. No identity is created. Wallet shows
   network funding separately from the identity session.
2. Write a post/reply → **Publish** (or **Reply**) prepares and saves signed
   bytes, without broadcast, and shows one line: the total fee in LTC and a
   **Confirm** button. Optional wire and hashtag inputs are structured protocol
   fields. Editing the draft discards the quote. See the
   [configuration sources](#configuration-and-state) for the fee policy; there is
   no automatic fee increase. Funding comes from the vault's BIP84 Litecoin
   Testnet account (`m/84'/1'/0'`, Identity/Vault v2): confirmed coins found on
   its external and internal chains (gap limit 20) are spent largest first, up
   to 50 inputs, and the change and return output go to the designated address
   `0/0`. The identity slot only signs as author and never funds.
3. **Confirm** approves that exact plan and begins submission. The commit is
   submitted first; once observed in the mempool or confirmed, its reveal is
   submitted in the same pass. The line disappears once both transactions are
   sent; Wire keeps reconciling in the background until both confirm. A source
   error or rejection stops with its reason and a **Retry** button (and
   **Abandon** if nothing reached the network); no fee bump occurs.
4. Completion refreshes the confirmed feed. **Refresh confirmed feed** also
   advances the index explicitly. The confirmed index never stores mempool
   records; see [unconfirmed records](#unconfirmed-records).

## Unconfirmed records

Settings → **Show records from the mempool** (off by default, kept in WebView
localStorage) adds Wire records still waiting in the mempool to the feed, marked
**pending**. Every five seconds Wire lists the mempool, fetches each new
transaction once, and keeps only URMA reveals whose author proof verifies
against their commit. A pending record turns into a normal one when it confirms
and the feed refreshes; one that leaves the mempool unconfirmed disappears. A
pending reply nests under its parent like any reply. Pending records live only
in memory: they never enter the confirmed index, and opening one shows nothing
until it confirms.

The mempool list comes from the chain sources, whatever serves the feed:
Litecoin Core (`getrawmempool`, local or the explicit Local node choice) and the
public HTTP sources (Esplora `mempool/txids`, the JSON-RPC gateway). Electrum
servers, the P2P light client and URMA Node cannot list the mempool; when only
those are reachable the setting shows the source error. At most 200 new
transactions are inspected per poll.

Pause stops after the in-flight request. Closing Wire stops following; reopen
and resume the saved signed plan without unlocking again. Locking the identity
prevents new signing but does not revoke an already approved signed plan.
The app admits one unfinished approved publication and one instance per data
directory, preventing concurrent app plans from reusing funding.

## Configuration and state

The configuration entry points are [app-core](../app-core/src/config.rs),
[app-engine](../app-engine/src/config.rs) and [app-ui](../app-ui/src/config.rs).
Consult these sources for changing bounds, defaults and timing; the flows below describe
their behavioral purpose.

Reads `URMA_CONFIG`, or `~/.config/urma/config.json`: `rpc_url`, `node_auth_file`,
`vault`, `unlock_file`. RPC/auth environment overrides are `URMA_RPC_URL` and
`URMA_NODE_AUTH_FILE`. Settings are loaded at startup; restart after changing
configuration.

The chain source is an explicit choice in Settings, stored in app-data
`source.json`. **Public sources** needs no node, cookie, API key or paid plan:
the shared `urma-runtime` transport routes each read through the Litecoin
Testnet provider set, in order, a peer-to-peer light client, the Electrum-LTC
servers, the litecoinspace Esplora API and the JSON-RPC gateway. A request goes
to one provider at a time; failover moves to the next provider, and the
Settings page lists the set the open connection actually holds. **Local node**
is offered only when `rpc_url` and `node_auth_file` are both configured, uses
cookie authentication on a loopback address, and a configured RPC failure never
silently redirects to a public endpoint. Every source must return the Litecoin
Testnet genesis hash before it serves anything. Wire holds one connection per
run and reconnects only after a transport failure or a source change.

Under app-data `runtime/` the public transport keeps its own state:
`electrum-pins/<host>_<port>.sha256` holds the SHA-256 of each Electrum server's
leaf certificate, pinned on first use and refused if it changes (remove the file
only after confirming the new fingerprint with the operator), and the light
client's header cache. The Connection page shows the evidence tier behind the
last check: `Local validating node` (your node validated the blocks),
`Light-client inclusion` (a proof-of-work header chain includes the
transaction) or `Public provider observation` (third-party servers reported
inclusion; bytes were verified against the requested ids, no block was
validated locally), together with the light client's header and peer counts
while it syncs. Record signatures are verified at every tier.

Default vault/unlock paths match the CLI. Passwords are never saved by Wire.
Only Identity/Vault v2 files (12-word phrase) unlock; v1 vaults are refused and
an older imported `identity.urma` is ignored, not converted.
The Identity page copies a validated encrypted vault into app-data `identity-v2.urma`,
subject to the shared URMA vault format validation. Import requires its password,
validates its active author before writing, and never overwrites an existing
imported vault or the CLI vault. The imported copy takes precedence for future
unlocks. Its keys
stay in the running session until Lock or process exit; reloading the frontend
only reads public session status and never automatically unlocks. A source or
balance failure does not lock the identity or change its author.
State is under Tauri's platform app-data directory for `org.urma.wire`:
`plans/<plan-id>/plan.json`, exact approval, progress journal, `active.json`,
`ltc-testnet-index-v3.json` (index format 3, which records each reveal's return
script; an older index file is ignored and history is resynced) and
`ltc-testnet-start.json`. Existing prototype
`wire_store.json` is left untouched and not used as verified chain evidence.

First sync starts within a bounded recent range before the current tip and remembers
that height. The engine owns the sync run: `StartSync` launches one catch-up to the
tip (blocks downloaded in parallel, applied in order, index persisted per batch) and
`SyncStatus` reports per-block progress; the UI only polls and renders it. This is a
recent-history feed, not full-chain search.
The library verifies record/author proofs and rolls back orphaned index entries.
Posts and replies support an inert Markdown subset (text/code/paragraphs and
external image references); HTML is displayed as text. Bitcoin UI integration
remains separate work.

## Public profiles and pixel-art avatars

Identity includes a nickname editor and a user-authored 16×16 drawing editor,
using exactly the reference 16-color palette. Click/tap paints pixels; mouse
dragging paints continuously. The preview and signed review use the same inert
renderer. Kind-05 nicknames contain 0–128 UTF-8 bytes, in an 8–136-byte record;
transaction overhead is additional. The editor counts octets and disables review
above the shared core limit. Nicknames preserve submitted UTF-8 exactly, including
empty names, U+0000, whitespace and distinct NFC/NFD spellings; there is no
truncation or normalization. Names are not unique. Empty/missing names fall back
to the key fingerprint.

Nickname (kind 05) and avatar (kind 06) updates each use the existing atomic
quote → exact fee approval → commit/reveal → resume flow. The avatar is 128
packed bytes containing 256 palette indices, scanned row-major from the top-left.
Each byte stores the first pixel in its high nibble and the next in its low nibble,
in a 136-byte URMA record; transaction
overhead is additional. Review decodes the saved signed reveal, including its
frozen avatar pixels, rather than reading the current editor. Both native and
browser publishing use the same record construction and review code.

Profiles and avatars are resolved independently from the selected network's
verified confirmed index, ordered by block height and transaction position.
The latest supported nickname and avatar each win; a newer legacy record is
omitted and does not replace an earlier supported value.
Every feed refresh rebuilds this disposable state, including after reorgs, and
cards/replies/profile pages use it with fingerprint/identicon fallbacks. This
inherits the current bounded history range: older identity updates outside that
range are unavailable. **Load confirmed profile** copies the indexed state into
the editor; drafts remain local and are not saved across page changes.

The palette and dimensions come from `urma_core::format::Urma`; Wire reuses the
core packing helpers. All 128-byte payloads are pixels, including bytes resembling
image-file signatures. No image codec, SVG input, sniffing or automatic format
conversion participates in avatar rendering. The renderer generates inert SVG
rectangles in a 16×16 view box. The separate fallback identicon keeps its 32×32
geometry. The pixel encoder refuses indices outside 0–15.

Cached legacy profile records require the exact V0 kind-05 prefix, valid UTF-8
and a 129–32760-byte body. Their original index entries remain unchanged and are
omitted with a diagnostic; only supported nicknames participate in identity
selection. Other malformed headers, flags, UTF-8 and out-of-range lengths remain
errors. An oversized nickname in a directly stored identity displays the key
fingerprint. Loading it into the editor retains the exact text, reports its byte
count and warns; review remains disabled until the operator explicitly edits it.
The original cached identity remains unchanged.

Cached legacy avatar records with the exact V0 kind-06 prefix and 512-byte payload
(520 bytes including the prefix) are retained unchanged and omitted from feed
projection with a diagnostic. Valid posts, replies, nicknames and current avatars
still load after restart in both engines. Other malformed records remain errors.
A stored identity containing unsupported avatar bytes uses the identicon with an
unsupported-avatar notice; loading it into the editor shows a blank canvas with a
notice and leaves the stored bytes unchanged. A saved legacy avatar or oversized
profile plan fails strict validation and is dropped before any network call; see
[saved publication recovery](#saved-publication-recovery).

## Saved publication recovery

A saved plan that Wire can no longer read or validate (for example one written
by an older format) is dropped with a logged warning: the desktop removes
`active.json` and that plan's directory, the browser removes its stored plan.
Publishing continues with a fresh review. Wire never resumes, re-signs or
migrates such a plan; transactions it already submitted stay on chain, and a new
plan that selects the same funding is refused by the network if that funding was
already spent.

## External images

Published photo references use the existing Markdown image representation
internally; the **Add photo** control does not require writing that syntax.
Native checked HTTP/HTTPS fetching, including
public IP hosts. DNS answers must all be public unicast; each redirect destination
is resolved and checked separately. Connections are pinned to those addresses,
environment proxies are disabled, and redirects are bounded. HTTP connection and
request timeouts are configured; OS DNS resolution is separate and can outlast
the request timeout. No cookies, credentials or referring post URL are
sent. Fetching still exposes the user's network address and request metadata.

PNG, JPEG, WebP, GIF, BMP, ICO and TIFF are enabled on desktop and Android;
desktop also supports AVIF. Android reports AVIF as an unsupported image format.
A response must declare an `image/*` type, but the decoder is chosen from the
signature, so a host that transcodes or mislabels its body cannot select the
codec. Download,
dimensions, total pixels, decoder allocation and re-encoded output are bounded;
see the [configuration sources](#configuration-and-state).
Limits also apply while reading image dimensions. Rendering re-encodes inert
pixels and discards source metadata/animation; SVG and arbitrary embeds are
unsupported. Image references, concurrent requests and displayed data URLs plus estimated
RGBA pixels have bounded budgets (not a process-wide memory guarantee).

The **Don't autoload external images** preference persists in WebView
localStorage; disabled images show an explicit **Load image** control. When
autoload is enabled, opening a post can contact its image host and expose the
user's network address.
Preference failures are reported and a failed initial read disables autoload.
The browser build explicitly disables image fetching because browsers cannot
enforce DNS pinning and redirect checks; HTTPS pages additionally explain HTTP
mixed-content refusal. It uses no media proxy. Profile/avatar publication and
pixel rendering still work in both runtimes.

Wire retains no HTTP, disk or application media cache. Only currently displayed
images live in memory. **Refresh image**
discards its display and requests the current resource again; **Clear displayed
media** removes displayed data and invalidates in-flight results. Hosts can change
or remove images; Wire does not claim historical preservation.

## Wires, hashtags and local follows

A wire such as `/rosint` is an open channel, with no owner or registration;
using a label never proves affiliation. Each post/reply can name one wire and
up to eight hashtags such as `#investigation`, shared across wires on the same
network. Leaving both fields empty keeps legacy kind 02/04 encoding. Structured
records use new kinds 0A/0B. Names use 1–32 lowercase ASCII letters/digits, `-`
or `_`, beginning with a letter/digit; the UI lowercases label input and sorts
and deduplicates hashtags before exact approval. Text remains unchanged.
Cards show a carried label where the text writes it, and that word follows or
unfollows it; only labels carried without appearing in the text are shown as
separate chips. This is display only: the signed text and topics are unchanged.

Global feed includes all records and clears both optional filters. Separate wire
and hashtag controls work independently: either alone filters that dimension;
both together require a record to match both (AND). Selecting or clearing one
does not change the other. Each Follow button subscribes only to its individual
wire or hashtag, never a combined filter. Following remains a separate view of
the union of locally followed labels (OR). Filters apply before taking the
bounded latest results. Follows persist in WebView localStorage, keyed by chain and
network name, with a bounded local capacity; no transaction or payment is involved.
They are tied to this local WebView profile. Storage failures are shown.
The initial network options are fixed and only Litecoin Testnet is enabled.

Index format 2 rescans a format-1 cache from its recorded starting height, because
old readers could have skipped the newly allocated kinds. This occurs on the
next explicit sync, uses the normal bounded slices and retains chain validation.
No full-chain history or reduced blockchain scanning cost is implied.

## Replies in the feed

A reply whose parent is shown in the feed sits under that parent instead of
the top-level stream. The newest replies appear first, up to Settings →
**Replies shown per post** (0 keeps them collapsed, persisted in
WebView localStorage); **View all N replies** opens the whole thread. A reply
to a reply stays inside its parent reply's thread. A reply whose parent is
outside the indexed history, or filtered out, stays top-level and names the
record it answers. Grouping is display only: records and the index are unchanged.

## Linux desktop inspection check

After building the dependencies, an isolated desktop smoke can exercise native
IPC and visible Identity/Settings navigation without a user vault or live chain
source:

```sh
python3 scripts/desktop_smoke.py --evidence target/desktop-smoke
```

It launches `cargo tauri dev --no-watch` with disposable HOME/XDG paths, a missing
vault and an explicit loopback source, using private Xvfb/DBus. It requires Python
GObject/AT-SPI bindings, Xvfb, `dbus-run-session`, `xdotool` and ImageMagick's
`import`. Its overall deadline and process-group cleanup bound the run. An
already-built application can be selected with `--binary /absolute/path/to/app`.
Logs, accessibility observations and screenshots go to the evidence directory;
review the screenshots separately. This is a reproduction command, not a recorded
PASS, packaged-platform certification or a signing/publication test.

## Local readiness verification (11 October 2026)

Against clean public core `73f5972a0a3f9c06b3047fa7c669539f5f3f06ce`, the
readiness changes were checked on Linux with Rust 1.98.0, Tauri CLI 2.11.2,
Trunk 0.21.14, GTK 3.24.52 and WebKitGTK 2.52.6:

- All core/engine/UI native tests passed: **107 passed, zero failed or ignored**.
- Ordinary WASM and native desktop builds passed. The offline worker regression
  passed; the fixture WASM build and real Chromium/IndexedDB legacy-profile gate
  passed, including actionable recovery text, preserved saved bytes and no
  mocked submissions.
- Actual `cargo tauri dev --no-watch` passed the isolated desktop smoke. The
  served CSP header had a per-response nonce matching the bootstrap HTML.
  Identity, native session response and Settings/local-node controls worked;
  screenshots were reviewed.
- A debug `cargo tauri build --no-bundle` using an offline debug frontend also
  passed the same desktop smoke from embedded assets with the packaged CSP.
  This is not a release-optimized binary, installer or signing certification.
- Lawkeeper still reports the same **29 pre-existing findings** as baseline
  `637f47ab`; normalized rule/path diagnostics are unchanged. It is not a full
  Lawkeeper PASS. These structural findings remain maintenance work.

Both desktop checks used disposable HOME/XDG paths and an unavailable loopback
node; no operator vault or live chain was used. Browser sources were mocked.
These checks do not establish live-provider availability, mainnet readiness or
exhaustive CSP security. Desktop test processes were stopped after verification.
This evidence predates the public experimental source release. Publication of
the current application, including its bundled branding, was authorized on
11 October 2026; see [NOTICES.md](../NOTICES.md). It does not turn these local
checks into certification of other platforms or live providers.

## Focused local checks

The commands below are reproduction instructions, not evidence that checks were
rerun for this documentation update. Test coverage descriptions are recorded in
README revision `ef850ea09c6d50d352d35c4902e937c67424cafe`; they do not establish
the execution date or tested source revision.

Coin tab logos are bundled locally: Litecoin from
https://github.com/spothq/cryptocurrency-icons/blob/master/svg/color/ltc.svg
(CC0), and Bitcoin from
https://github.com/bitpay/bitcoin-brand/blob/master/bitcoin.svg.

The [UI manifest](../app-ui/Cargo.toml) records the Thaw and Leptos dependencies.
The network selector uses Thaw Menu;
the four choices remain visible while only Litecoin Testnet is enabled.

The sidebar logo is the sync gauge. The pulse travelling the U channel sits at
`verified / total` of the trace's arc length (`app-ui/src/components/brand.rs`),
so each block of a hundred advances it one hundredth; the fracture cuts seal as
the pulse passes them and the red break point turns green once the index is
complete to the tip. Under the logo a short status reads `X/Y` while verifying,
`Retrying in N s` while the engine backs off after a source failure (5 s doubling
to 60 s, eight attempts, progress kept), then `Up to date`, `History incomplete`
or `Sync failed`; the feed itself carries no sync text. Below it a person icon
counts the light client's connected peers, a globe counts ready remote sources
out of the configured ones, and a monitor labelled Local shows the local daemon.
Hovering each gives its ready, unavailable or cooldown status. The eagle lives in
`assets/circuit-eagle.svg`; `assets/urma-wire.svg` remains the self-contained
favicon with its own timed animation.

```sh
cargo test --workspace --locked
cargo check --workspace --locked
cargo check -p wire-app-ui --target wasm32-unknown-unknown --locked
lawkeeper
```

Crates: `app-core` holds the domain types and the typed `Request`/`Response`/`Fault`
contract; `app-engine` holds the single projection and record codec plus the
native (Tauri) and browser (IndexedDB/Esplora) backends; `app-ui` is Leptos only
and asks the engine through `wire_app_engine::client::ask`; `src-tauri` exposes one
`engine` command. `law.toml` declares the layer map and one `config.rs` sanctum
per crate; `scripts/law_report.sh` tallies violations by file and rule.

Tests use temporary files, public fixture identities and a loopback RPC mock;
they do not start a blockchain node or broadcast to a live network. They cover
exact approval, post/reply preparation, confirmation stages, read-only status,
restart, unchanged retransmission, rejection/source failure and feed projection.
Focused profile/media checks are `cargo test -p wire-app-engine --test live
--test public_identity --test media --locked`: exact profile/avatar approval and
resume, fixed bytes/palette, colliding names/latest ordering/rollback projection,
destination/redirect bounds, MIME/signature/decode/size failures and inert browser
capability errors. These are local fixtures, not live media-host acceptance or
visual UI validation.

Avatar checks also cover all EGA palette indices, first/last pixels and row/nibble
order, arbitrary 128-byte payloads including image signatures, the 16×16 editor
grid, legacy cache restart and dropping an approved legacy plan before any
network call.

Focused kind-05 checks use the shared codec and independent UTF-8 reference:

```sh
cargo test -p wire-app-engine --test profile --test cached_profile --test legacy_profile_pending --test public_identity --test live --test legacy_pending --offline --locked
cargo test -p wire-app-ui --test profile --offline --locked
cargo check -p wire-app-ui --target wasm32-unknown-unknown --offline --locked
```

UI state tests cover exact confirmed-name loading and the direct-cache display
fallback. The ordinary UI remains CSR. The browser gate exercises the actual
nickname textarea counter and Review enabled/disabled transition at ASCII
128/129 and 32/33/32 emoji (128/132/128 bytes), without submitting a quote.
NUL, CRLF and NFC/NFD exact-preservation assertions concern Rust editor state and
record bytes; they do not assert browser textarea newline preservation.

Native tests exercise exact write/read, mixed cache restart and dropping an
approved signed oversized plan before any network call. Serialized browser cache
tests exercise the shared projection, rather than IndexedDB cache runtime. These commands use local fixtures and establish no live-chain acceptance.

## Local-node detection verification (11 October 2026)

The current change passed 110 native core/engine/UI tests, including a running
20-block index scan that begins with a public fixture and switches to a local RPC
fixture without replacing the reader. The same router covers outage fallback,
cookie rotation and malformed-block rejection; separate cases refuse wrong-genesis
and initial-download nodes and verify the local icon does not inflate remote counts.
The core dependency passed 18 focused transport/private-scan regressions, including
construction of the standard pool with an additional local provider. These use
synthetic or loopback fixtures, not a live-chain acceptance claim.

The ordinary WebAssembly frontend and embedded debug desktop build also passed.
The full app Lawkeeper result remains 29 pre-existing violations, with no added
rule/path findings; the core dependency passes Lawkeeper.

The packaged desktop smoke passed with `scripts/desktop_smoke.py --binary PATH
--local-fixture --evidence DIRECTORY`: native IPC, session refresh, Settings and
the accessible local-node status were checked. The screenshot shows the monitor,
P2P and remote indicators together below the counter. This GUI fixture uses
explicit Local-node mode with a minimal loopback RPC server; live discovery
mid-sync is covered by the engine regression above, not claimed as a real-chain
GUI experiment. The fixture contains no downloadable block bodies.

## Gallery removal verification (11 October 2026)

At `7c14f79`, the development Components gallery, route, styles and exemptions
were removed; reusable application components were preserved. All 19 UI tests,
the ordinary WASM frontend and embedded debug desktop build passed. The isolated
desktop IPC/navigation smoke passed, and its screenshots confirmed the remaining
sidebar and Identity page. This is additional UI evidence, not a new full-workspace
or live-network acceptance run.

## Public source preparation (11 October 2026)

The publication candidate `32b5ad1` (application code unchanged from `7c14f79`)
was exported using only its 217 tracked files. Core
`a17014a03071c2b72a056d0b4289db007ac1feaf` was freshly cloned from the public
`urma-protocol/urma` repository into the documented sibling location. All 13
local Cargo packages resolved within those two source trees. No private checkout
or private Cargo configuration was needed.

With the installed toolchain and an existing dependency/build cache, this
ordinary WASM plus embedded debug desktop build passed:

```sh
CARGO_BUILD_JOBS=2 CARGO_NET_OFFLINE=true NO_COLOR=1 TRUNK_SKIP_VERSION_CHECK=true \
cargo tauri build --debug --no-bundle --config \
  '{"build":{"beforeBuildCommand":"python3 scripts/frontend.py build --offline"}}'
```

A shared `CARGO_TARGET_DIR` reused the local build cache; it is not a source
dependency. First-time users should follow the online setup in the README;
`--offline` requires dependencies and frontend tools to have been downloaded.
Both Cargo lockfiles and every tracked application file remained unchanged.

The resulting executable passed `scripts/desktop_smoke.py --binary PATH
--local-fixture --evidence DIRECTORY` on a disposable profile. Native session IPC,
Identity/Settings navigation and the local-source indicator were exercised;
screenshots were reviewed and owned processes stopped. This check used a minimal
loopback RPC fixture, not a live chain. GitHub's Markdown renderer accepted the
README, including its headings, code blocks and architecture table; local document
links and anchors were checked. Release-optimized installers and other platforms
were not tested in this source-publication step.
