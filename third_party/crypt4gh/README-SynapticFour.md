# crypt4gh (SynapticFour fork)

Vendored from [crypt4gh 0.4.1](https://crates.io/crates/crypt4gh) (EGA-archive/crypt4gh-rust).
Apache-2.0. Upstream statements found in this tree, and the unchanged license text, are in [NOTICE](NOTICE).

Upstream author recorded in `Cargo.toml`: `Roberto <roberto.ariosa@crg.eu>`.
Upstream docs authors recorded in `docs/book.toml`: `EGA-CRG System Developers`.
No separate copyright line was present in the Rust sources.

## Why forked

Upstream `crypt4gh` 0.4.1 depends on `rust-crypto`, which is unmaintained and flagged
critical ([RUSTSEC-2022-0011](https://rustsec.org/advisories/RUSTSEC-2022-0011.html)).
`rust-crypto` is only used for OpenSSH private-key decryption (AES-CTR/CBC + scrypt/bcrypt KDF);
Crypt4GH payload encryption already uses libsodium ChaCha20-Poly1305.

This fork replaces `rust-crypto` with RustCrypto `aes`, `ctr`, `cbc`, `scrypt`, and `bcrypt-pbkdf`.

Solum depends on this path from the workspace `Cargo.toml` (`crypt4gh = { path = "third_party/crypt4gh" }`). The Apache-2.0 text in `LICENSE` is unchanged.
