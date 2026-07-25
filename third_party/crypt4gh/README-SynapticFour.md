# crypt4gh (SynapticFour fork)

Vendored from [crypt4gh 0.4.1](https://crates.io/crates/crypt4gh) (EGA-archive/crypt4gh-rust).

## Why forked

Upstream `crypt4gh` 0.4.1 depends on `rust-crypto`, which is unmaintained and flagged
critical ([RUSTSEC-2022-0011](https://rustsec.org/advisories/RUSTSEC-2022-0011.html)).
`rust-crypto` is only used for OpenSSH private-key decryption (AES-CTR/CBC + scrypt/bcrypt KDF);
Crypt4GH payload encryption already uses libsodium ChaCha20-Poly1305.

This fork replaces `rust-crypto` with RustCrypto `aes`, `ctr`, `cbc`, `scrypt`, and `bcrypt-pbkdf`.

Wired via `[patch.crates-io]` in the Ferrum workspace root `Cargo.toml`.
