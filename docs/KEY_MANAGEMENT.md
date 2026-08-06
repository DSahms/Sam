# Key Management — Unlock and Recovery

This document specifies how Sammy protects a vault's data-encryption key,
how the owner unlocks a vault, and how the owner can recover a vault when
the master passphrase is lost. The design principle is simple: there is
exactly one place that holds the canonical key material, exactly two paths
to unwrap it, and no third path exists for anyone — including the vendor.

## The keys and their relationships

Three pieces of key material are in play for every vault:

- **Vault Data-Encryption Key (VDEK).** A random 256-bit symmetric key
  generated at vault creation by the platform CSPRNG. This single key
  decrypts the SQLCipher database and the encrypted source files on disk.
- **Key-Encryption Key (KEK).** Derived from the owner's master passphrase
  via Argon2id. The KEK is never stored; it exists only in memory during an
  unlock.
- **Recovery Key-Encryption Key (RKEK).** Derived from a separate
  high-entropy recovery key, used to wrap a second copy of the VDEK so the
  vault can be opened without the passphrase.

The VDEK is stored on disk **only** in wrapped form, twice: once under the
passphrase-derived KEK and once under the recovery-derived RKEK. The
plaintext VDEK exists in memory only while a vault is unlocked.

```
                  passphrase --> Argon2id --> KEK --
                                                 \
                                                  +--> wrapped VDEK (stored)
                                                  /
  recovery key  --> KDF (high-entropy) --> RKEK --
```

## What is stored on disk

The vault header (a small structure stored alongside the encrypted database)
contains:

- format version (so the unlock path can evolve)
- salt for the passphrase Argon2id derivation
- Argon2id parameters: `memory_cost`, `time_cost`, `parallelism`, output length
- wrapped VDEK under KEK (`ciphertext + nonce + MAC`)
- salt/parameters for the recovery key KDF
- wrapped VDEK under RKEK (`ciphertext + nonce + MAC`)
- vault ID and template marker

## What is never stored

- The plaintext master passphrase — never.
- The plaintext VDEK — never at rest; only in memory while unlocked.
- The plaintext recovery key — never.
- The KEK or RKEK — never at rest; derived on demand and zeroized.
- Any passphrase hint — never. (Hints are an attack surface and the
  recovery key exists precisely so a hint is not needed.)

## Unlock flow

1. Read the vault header.
2. Collect the master passphrase from the owner (UI).
3. Derive KEK via Argon2id with the stored salt and parameters.
4. Attempt to unwrap the VDEK using the KEK. A MAC failure here means the
   passphrase is wrong; return a generic "incorrect" error with no leak of
   which step failed.
5. On success, open the SQLCipher database with the VDEK and decrypt source
   files on demand. The KEK is zeroized; the VDEK remains in memory only as
   long as the vault is unlocked.
6. Locking the vault zeroizes the VDEK and closes the database connection.

## Recovery flow

Recovery uses the second wrapping:

1. Owner chooses "I forgot my passphrase" and provides the recovery key.
2. Derive RKEK from the recovery key.
3. Unwrap the VDEK with RKEK.
4. With the VDEK in memory, allow the owner to set a **new** master
   passphrase. This derives a new KEK and re-wraps the VDEK under the new
   KEK. The recovery wrapping is left intact (the same recovery key still
   works) unless the owner explicitly rotates it.

A correct recovery key plus a forgotten passphrase therefore recovers the
vault completely; no data is lost.

## Recovery key generation and confirmation

The recovery key is generated at vault creation:

- High entropy: at least 128 bits of CSPRNG output, rendered in a
  human-transcribable group form (e.g., grouped base32 words).
- **Displayed once**, in full, at creation time, with a clear warning that
  it cannot be shown again and that losing both it and the passphrase means
  the vault is permanently unrecoverable.
- **Confirmed** by requiring the owner to retype or re-select the key before
  vault creation completes. A key that has not been confirmed does not
  produce a usable vault.

## No vendor backdoor

There is no escrow, no master key, no support path, and no online service
that can unwrap the VDEK. The KDF runs locally; the wrapping is local; the
MAC verification is local. The vendor cannot decrypt a vault even given a
full backup file. This is a deliberate property: it means a stolen backup
file is useless to anyone who lacks either the passphrase or the recovery
key, including the developer.

## If both credentials are lost

If the master passphrase is forgotten **and** the recovery key is lost, the
VDEK cannot be unwrapped by any means available to anyone. The vault is
**permanently unrecoverable**. The owner can delete the vault to reclaim
disk space, but its contents are gone. Sammy must surface this clearly and
without false comfort; offering a "support-assisted recovery" option would
be a lie and is forbidden.

## Windows Hello (optional future convenience)

Windows Hello may be added later **only** as a convenience wrapper around an
existing key — never as a replacement for, or a third independent unwrapping
path for, the VDEK:

- On unlock, Windows Hello releases a stored, OS-protected copy of the
  passphrase (or of the VDEK itself) after biometric/PIN confirmation.
- The released material is then used in the normal unlock flow above.
- Enrolling Windows Hello requires a current successful passphrase unlock;
  disabling it removes the OS-stored copy and leaves passphrase + recovery
  unlock fully functional.

Windows Hello is not a recovery mechanism. If the device is lost, the
Hello-protected copy is lost with it, and the owner must use the passphrase
or the recovery key on a new machine.
