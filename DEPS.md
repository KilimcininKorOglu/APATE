# Dependency Audit and Justification

This document records the rationale, license, and risk assessment for every
production dependency declared in `Cargo.toml`. Dev dependencies used only
during testing and benchmarking are listed separately. The supply chain policy
that governs additions and updates follows at the end.

---

## Production Dependencies

### Cryptography and Key Material

| Crate            | Version | License        | Purpose                                                    | Replaces          |
|------------------|---------|----------------|------------------------------------------------------------|-------------------|
| `chacha20poly1305` | 0.10.1  | Apache-2.0 / MIT | Primary AEAD cipher. Used in `crypto::aead` for all data-plane encryption and decryption via `encrypt_chacha20poly1305` / `decrypt_chacha20poly1305`. | `ring` AEAD path  |
| `aes-gcm`        | 0.10.3  | Apache-2.0 / MIT | Secondary AEAD cipher. Available as an alternate AEAD for environments where AES hardware acceleration is present. | `ring` AES-GCM path |
| `x25519-dalek`   | 2.0.1   | BSD-3-Clause   | X25519 Diffie-Hellman key exchange. Used in `crypto::kx` for ephemeral key exchange during the Noise handshake (`derive_public_key`, `derive_shared_secret`). | `ring` key agreement |
| `ed25519-dalek`  | 2.2.0   | BSD-3-Clause   | Ed25519 digital signatures. Used in `auth::common_crypto` for signing and verifying `AuthProof` messages during the handshake, and for `StaticKeyBackend` / `CertificateBackend` verification. | `ring` signing |
| `ml-kem`         | 0.2.3   | Apache-2.0 / MIT | ML-KEM (CRYSTALS-Kyber, FIPS 203) post-quantum key encapsulation. Enables the hybrid X25519 + ML-KEM key exchange when `crypto.post_quantum = true`. | No prior equivalent |
| `blake3`         | 1.8.4   | Apache-2.0 / CC0-1.0 | BLAKE3 hash and PRF. Used in `crypto::kdf` for transcript hashing inside `SymmetricState` (`mix_hash`, `mix_key`) and for deriving session keys from the shared secret. | `ring` SHA-2 / HKDF |
| `rand_core`      | 0.10.0  | Apache-2.0 / MIT | Core random-number-generator traits. Required by `x25519-dalek`, `ed25519-dalek`, and `ml-kem` for key generation. | —                 |
| `rand_chacha`    | 0.10.0  | Apache-2.0 / MIT | ChaCha20-based CSPRNG. Used in `crypto::rng` to provide deterministic, seedable randomness for key generation and nonce derivation. | `ring` SecureRandom |
| `zeroize`        | 1.8.2   | Apache-2.0 / MIT | Guaranteed zeroing of secret key material on drop. Applied via `#[derive(Zeroize)]` on all key structs in `crypto::kx`, `crypto::aead`, and auth backends to prevent key material persisting in freed memory. | — |
| `subtle`         | 2.6.1   | BSD-3-Clause   | Constant-time comparison primitives. Used in `noise::handshake` (`signature.ct_eq`) and auth backends to prevent timing side-channels during secret comparison. | — |

**Rationale for not using `ring`:** The `ring` crate bundles native C/assembly code and
requires a C toolchain in the build graph. Apate targets a pure-Rust dependency tree
(enforced by `deny.toml`) to simplify cross-compilation to Windows, FreeBSD, and musl
targets. All cryptographic primitives covered by `ring` are replaced by the RustCrypto
ecosystem crates listed above.

---

### Platform and OS Interfaces

| Crate          | Version | License    | Purpose                                                                   | Replaces |
|----------------|---------|------------|---------------------------------------------------------------------------|----------|
| `libc`         | 0.2.184 | Apache-2.0 / MIT | POSIX system call bindings. Used in `tunnel::tun_linux` and `tunnel::tun_macos` to open `/dev/net/tun` (Linux) and `/dev/tun` (macOS), configure TUN interfaces via `ioctl`, and set socket options. | — |
| `windows-sys`  | 0.61.2  | Apache-2.0 / MIT | Windows API bindings (Win32 sockets, I/O completion ports, foundation types). Used in `tunnel::tun_windows` for WinTUN adapter management and in `runtime::backend::iocp` for the IOCP event loop. Target-gated: `cfg(windows)` only. | — |

---

### Data Structures and Utilities

| Crate        | Version | License    | Purpose                                                                                  | Replaces              |
|--------------|---------|------------|------------------------------------------------------------------------------------------|----------------------|
| `hashbrown`  | 0.16.0  | Apache-2.0 / MIT | `HashMap` and `HashSet` with SwissTable implementation. Used in `auth::backend::AuthCoordinator` and `routing::table::RouteTable` for O(1) lookups without requiring `std` allocator guarantees everywhere. | `std::collections::HashMap` |
| `bytes`      | 1.10.1  | MIT        | `Bytes` and `BytesMut` zero-copy buffer primitives. Used in `util::buf` via `ByteWriter` and `ByteCursor` for frame encoding and decoding, and in `transport::frame` to construct and parse wire frames without unnecessary copies. | — |
| `thiserror`  | 2.0.11  | Apache-2.0 / MIT | Procedural macro for ergonomic `std::error::Error` derivation. Applied to all error enums: `ConfigError`, `RuntimeError`, `FrameError`, `TransportError`, `SecurityError`, `AuthError`, `CryptoError`, `ProfileError`, `RouteTableError`. | Manual `Display` impls |

---

### Observability

| Crate                | Version | License    | Purpose                                                                    | Replaces |
|----------------------|---------|------------|----------------------------------------------------------------------------|----------|
| `tracing`            | 0.1.41  | MIT        | Structured, leveled event and span instrumentation. Used throughout all modules for `trace!`, `debug!`, `warn!`, and `error!` calls. Provides the `EventCode`-tagged log lines emitted by `telemetry::log`. | `log` crate |
| `tracing-subscriber` | 0.3.20  | MIT        | `tracing` subscriber implementations. Provides the formatted subscriber initialized at startup in `telemetry::log` for writing structured log lines to stderr. | — |

---

## Dev Dependencies

| Crate       | Version | License    | Purpose                                                                                         |
|-------------|---------|------------|-------------------------------------------------------------------------------------------------|
| `proptest`  | 1.9.0   | Apache-2.0 / MIT | Property-based testing. Used in `tests/property/frame_roundtrip.rs` and `tests/property/parser_fuzz_harness.rs` to generate arbitrary inputs and verify invariants (frame encode/decode round-trip, config parser rejection of invalid inputs). |
| `criterion` | 0.8.2   | Apache-2.0 / MIT | Statistical micro-benchmark harness. Used in `benches/packet_path.rs`, `benches/crypto_wrappers.rs`, and `benches/handshake.rs` to measure packet processing throughput, AEAD latency, and handshake round-trip time. |

---

## Supply Chain Policy

### Allowed Licenses

Only the following SPDX license identifiers are permitted. Any dependency
that introduces a license outside this set must be reviewed and explicitly
allowed in `deny.toml` before merging.

- Apache-2.0
- MIT
- BSD-3-Clause
- CC0-1.0
- ISC

GPL, LGPL, AGPL, SSPL, and proprietary licenses are unconditionally denied.

### Tooling Enforcement

`cargo deny` is executed in `security.yml` on every pull request and on the
release workflow. The `deny.toml` at the repository root configures:

- `[licenses]` — allowlist matches the list above; `deny = ["GPL-*", "LGPL-*"]`
- `[advisories]` — blocks crates with open RustSec advisories rated `CVSS >= 7.0`; lower-severity advisories surface as warnings and must be triaged within 14 days
- `[bans]` — duplicate major versions of cryptographic primitives are denied to prevent confusion attacks where two versions of the same primitive coexist with different behavior
- `[sources]` — only `crates.io` is an allowed registry; git and path sources are denied in production builds

### Version Pinning

All production dependencies specify an exact minor version floor (e.g., `"0.10.1"`),
not a wildcard range. Patch-level updates may be merged after CI passes. Minor or
major version bumps require a dedicated PR with an updated entry in this file.

### Adding a New Dependency

Before opening a PR that adds a dependency, the author must:

1. Verify the license is on the allowed list.
2. Check `cargo audit` for known advisories.
3. Confirm the crate is not a duplicate of functionality already covered by
   an existing dependency.
4. Add or update the entry in this file with version, license, purpose, and
   what (if anything) it replaces.
5. Run `cargo deny check` locally and confirm it exits 0.

### Removing a Dependency

When a dependency is removed, its entry must be deleted from this file in the
same commit. Removing a crate that was listed as replacing another crate
requires documenting what now covers that functionality.

### Audit Cadence

A full `cargo audit` and `cargo deny check` run is required:

- On every pull request (enforced by CI)
- On the first working day of each calendar month (scheduled workflow)
- Immediately after any upstream advisory affecting a dependency is published
