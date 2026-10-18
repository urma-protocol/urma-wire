# Identity: state model, onboarding, recovery

## One model

The 12 words are the identity. Everything else is a copy.

```
12 words ──BIP39──▶ seed ──BIP32──▶ root ──slot 0──▶ author key (signs)
                                        └──BIP84──▶ funding account (pays)
```

A **vault** is the encrypted copy of that keyring kept on one device. Its
password protects the copy at rest and nothing more: it is never needed to
rebuild the identity from the words. Vault v2 wraps a random data key twice,
once under argon2(password) and once under a key derived from the phrase, so
a vault can also be opened with its words; Wire does not rely on that, it
simply rebuilds the keyring from the words and seals a fresh vault.

The engine holds the only session truth:

```
Identity = Locked | Unlocked(wallet)          in memory, per process
Vault    = Missing | Local | Configured{..}   on disk / IndexedDB
IdentityStatus { session: Session, vault: Vault }
```

`Local` is the app-data copy (`identity-v2.urma`, IndexedDB `identity-v2`).
`Configured` is the CLI vault named in `~/.config/urma/config.json` (desktop
only), used when no local copy exists. A local copy always shadows it.

Every identity request answers with the full `IdentityStatus`:

| request                               | effect                                   |
|---------------------------------------|------------------------------------------|
| `IdentityStatus`                      | read                                     |
| `CreateIdentity{password, replace}`   | new words → vault → local copy → unlocked, returns the words once |
| `RecoverIdentity{phrase, password, replace}` | words → vault → local copy → unlocked |
| `ImportIdentity{vault, password, replace}`   | checks the password → local copy → unlocked |
| `Unlock(credential)`                  | opens the local or configured copy       |
| `Lock`                                | drops the keys                           |

`replace` must be `true` to overwrite an existing local copy; the engine refuses
otherwise. The write is atomic (temp file + rename, or an IndexedDB `put`), and
the session switches only after the write succeeds, so a failed write leaves the
previous copy and session intact.

Both engines mint the vault through one shared function
(`app-engine/src/vault.rs`): phrase → `Keyring::create(phrase, "wire")` →
`UnlockedVault::create(.., password)` → sealed bytes + words. The native and the
browser backend differ only in where the bytes land.

## UI state

The interface keeps one signal for identity:

```
Identity = Reading | Unavailable(reason) | Known(IdentityStatus)
```

There is exactly one writer after a mutation, `AppState::identity_is(status)`,
fed by the engine's answer. It resets the wallet summary and requests a new one
when unlocked. No optimistic edits, no second fetch after a mutation. The
startup read carries an epoch: an answer from before a later mutation is
dropped, so a slow status read can never overwrite a newer state.

Wallet answers already carry the session they were requested for and are
discarded when the session moved on. Publishing is unchanged: the engine's
single active plan is the truth, and `state.publication` mirrors it.

The profile editor adopts the confirmed nickname and avatar automatically
whenever the signer or the indexed identities change, as long as the user has
not diverged from the last adopted baseline. A diverged editor keeps the user's
work and offers **Reset to confirmed**.

## Onboarding

The Identity page shows one screen at a time, chosen from the status:

```
Reading ─────────────▶ spinner
Unavailable ─────────▶ reason + Retry
Locked, Missing ─────▶ Welcome: [Create] [I have 12 words] [Import a vault file]
Locked, Local/Conf. ─▶ Unlock: password [Unlock] (+ [Use unlock file])
                        links: "Forgot the password? Recover with the 12 words"
                               "Use a different identity"
Unlocked ────────────▶ Identity card (fingerprint, key, profile link, Lock)
                        + Public profile (nickname, avatar)
```

Each entry step is one short form with a Back link:

- **Create**: choose a device password → `Create identity`. The 12 words are
  shown once with `I wrote them down`.
- **Recover**: paste the 12 words, choose a new device password → `Recover`.
- **Import**: pick the vault file, type its password → `Import`.

When a local copy exists the step says so, names the fingerprint it replaces,
and the button reads `Replace and …`. The request carries `replace: true`.

## Recovery without the password

Forgetting the device password loses nothing. From the Unlock screen, Recover
takes the 12 words and a new password, rebuilds the same keyring (same author,
same funding account) and seals a new vault under the new password, replacing
the local copy. The old password is never asked for. Tests cover: same author
from the words alone, the old password refused afterwards, the new one accepted,
and the refusal to overwrite without `replace`.
