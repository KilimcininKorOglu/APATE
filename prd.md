# Apate — Stealth VPN Protocol

**Version:** 1.0-draft
**Author:** KilimcininKorOglu
**Language:** Rust (`#[no_std]` compatible, minimal curated dependencies)
**Named after:** Ἀπάτη — Greek goddess of deceit and trickery

---

## 1. Vision

Apate is a stealth VPN protocol written in Rust with a **minimal, curated dependency** philosophy, engineered for three non-negotiable goals:

1. **Minimal dependencies** — only audited, best-in-class crates for security-critical primitives (crypto, platform abstraction); everything protocol-specific is hand-written
2. **Ultra-low latency** — sub-millisecond overhead per packet in steady state
3. **DPI evasion** — protocol traffic is indistinguishable from legitimate HTTPS/TLS 1.3 traffic under statistical and structural analysis

No tokio. No quinn. No rustls. No kitchen-sink frameworks. Audited crypto libraries, thin platform abstraction where needed, everything else from scratch.

### 1.1 Dependency Philosophy

The guiding principle: **"buy crypto, build protocol."**

| Layer              | Approach        | Rationale                                                  |
|--------------------|-----------------|-------------------------------------------------------------|
| Cryptography       | Crate (audited) | Hand-rolled crypto is a liability; use formally verified / audited implementations |
| Platform syscalls  | Crate (thin)    | `libc` for portability, avoid raw asm per-platform          |
| Async I/O          | Hand-written    | tokio is 200+ transitive deps; we need a minimal reactor    |
| Transport protocol | Hand-written    | Core differentiator, must be fully controlled               |
| Stealth / DPI      | Hand-written    | Core differentiator, no existing crate does this            |
| TUN interface      | Hand-written    | Thin ioctl wrapper, not worth a dependency                  |
| Config parser      | Hand-written    | TOML subset, < 300 LOC                                      |
| Framing / FEC      | Hand-written    | Tightly coupled to transport, custom requirements           |

**Hard rules:**
- Maximum **15 direct dependencies** in Cargo.toml (excluding dev-dependencies)
- Maximum **50 transitive dependencies** total
- Every dependency must be justified in a `DEPS.md` file
- No dependency may pull in tokio, hyper, or any async runtime
- `cargo audit` clean at all times
- `cargo deny` configured: no duplicate crate versions, no copyleft licenses

---

## 2. Anti-Goals

- Not a general-purpose VPN product with GUI/apps (that's a layer above)
- Not a Tor replacement (no onion routing, no anonymity network)
- Not RFC-compliant QUIC/TLS — we mimic their wire format but implement only what we need

---

## 3. Architecture Overview

```
┌──────────────────────────────────────────────────┐
│                   User Space                     │
│                                                  │
│  ┌─────────────┐  ┌──────────────────────────┐   │
│  │ apate-client│  │ apate-server             │   │
│  │             │  │                          │   │
│  │ ┌─────────┐ │  │ ┌─────────┐ ┌─────────┐ │   │
│  │ │ TUN I/O │ │  │ │ TUN I/O │ │ Web     │ │   │
│  │ └────┬────┘ │  │ └────┬────┘ │ Facade  │ │   │
│  │      │      │  │      │      └─────────┘ │   │
│  │ ┌────┴────┐ │  │ ┌────┴────┐             │   │
│  │ │ Routing │ │  │ │ Routing │             │   │
│  │ └────┬────┘ │  │ └────┬────┘             │   │
│  │      │      │  │      │                  │   │
│  │ ┌────┴─────────┴──────┴────┐             │   │
│  │ │    Apate Protocol Core   │             │   │
│  │ │ ┌───────┐ ┌───────────┐  │             │   │
│  │ │ │Crypto │ │ Shaping   │  │             │   │
│  │ │ └───────┘ └───────────┘  │             │   │
│  │ │ ┌───────┐ ┌───────────┐  │             │   │
│  │ │ │ FEC   │ │ Framing   │  │             │   │
│  │ │ └───────┘ └───────────┘  │             │   │
│  │ └────────────┬─────────────┘             │   │
│  │              │                           │   │
│  │ ┌────────────┴─────────────┐             │   │
│  │ │   Async I/O Runtime      │             │   │
│  │ │ (io_uring / epoll / kq)  │             │   │
│  │ └────────────┬─────────────┘             │   │
│  └──────────────┼──────────────┘            │   │
│                 │                                │
├─────────────────┼────────────────────────────────┤
│   Kernel        │                                │
│  ┌──────────────┴───┐                            │
│  │  UDP Socket       │                           │
│  │  TUN Device       │                           │
│  └──────────────────┘                            │
└──────────────────────────────────────────────────┘
```

### 3.1 Module Breakdown

```
apate/
├── src/
│   ├── main.rs                  # Entry point, CLI arg parsing
│   ├── lib.rs                   # Core library root
│   │
│   ├── runtime/                 # Custom async runtime (hand-written)
│   │   ├── mod.rs
│   │   ├── reactor.rs           # io_uring / epoll / kqueue reactor
│   │   ├── executor.rs          # Task executor (single + multi-thread)
│   │   ├── waker.rs             # Waker implementation
│   │   ├── timer.rs             # Timer wheel for timeouts
│   │   └── io_uring.rs          # io_uring syscall bindings (via libc)
│   │
│   ├── crypto/                  # Thin wrappers over audited crates
│   │   ├── mod.rs
│   │   ├── aead.rs              # AEAD trait + ChaCha20-Poly1305 / AES-256-GCM dispatch
│   │   ├── kx.rs                # Hybrid key exchange (X25519 + ML-KEM-768)
│   │   ├── sign.rs              # Ed25519 signature wrapper
│   │   ├── kdf.rs               # BLAKE3-HKDF key derivation
│   │   ├── rng.rs               # OsRng + ChaCha20Rng DRBG wrapper
│   │   └── zeroize.rs           # Re-export zeroize for key material
│   │
│   ├── noise/                   # Noise Protocol Framework (hand-written)
│   │   ├── mod.rs
│   │   ├── handshake.rs         # IK pattern handshake
│   │   ├── state.rs             # Handshake → Transport state machine
│   │   ├── cipher_state.rs      # Nonce management, rekeying
│   │   └── symmetric_state.rs   # MixHash, MixKey operations
│   │
│   ├── transport/               # Wire protocol & framing (hand-written)
│   │   ├── mod.rs
│   │   ├── frame.rs             # Frame encoding/decoding
│   │   ├── connection.rs        # Connection state machine
│   │   ├── stream.rs            # Multiplexed streams
│   │   ├── mode.rs              # Transport mode selection + auto-negotiation
│   │   ├── tcp.rs               # TCP transport mode (fallback)
│   │   ├── congestion.rs        # BBR-inspired congestion control
│   │   ├── loss.rs              # Loss detection & recovery
│   │   ├── fec.rs               # Reed-Solomon FEC (disabled in TCP mode)
│   │   ├── pacing.rs            # Packet pacing / send scheduling
│   │   └── migration.rs         # Connection migration (IP change)
│   │
│   ├── stealth/                 # DPI evasion subsystem
│   │   ├── mod.rs
│   │   ├── tls_camouflage.rs    # TLS 1.3 record layer mimicry
│   │   ├── quic_camouflage.rs   # QUIC v1 packet header mimicry
│   │   ├── client_hello.rs      # Fake ClientHello generation
│   │   ├── server_hello.rs      # Fake ServerHello generation
│   │   ├── padding.rs           # Statistical traffic shaping
│   │   ├── timing.rs            # Inter-packet timing jitter
│   │   ├── entropy.rs           # Entropy normalization
│   │   └── facade.rs            # HTTP(S) web facade for active probing
│   │
│   ├── tunnel/                  # TUN device management
│   │   ├── mod.rs
│   │   ├── tun_linux.rs         # Linux TUN via ioctl + raw fd
│   │   ├── tun_macos.rs         # macOS utun via sys/kern_control
│   │   ├── tun_freebsd.rs       # FreeBSD TUN
│   │   ├── tun_windows.rs       # Windows WinTUN via wintun.dll
│   │   └── packet.rs            # IP packet parsing (v4/v6 header)
│   │
│   ├── routing/                 # Packet routing
│   │   ├── mod.rs
│   │   ├── table.rs             # Routing table (LPC trie)
│   │   ├── split.rs             # Split tunneling rules
│   │   ├── dns.rs               # DNS interception / leak prevention
│   │   └── doh.rs               # Built-in DNS-over-HTTPS stub resolver
│   │
│   ├── auth/                    # Pluggable authentication (hand-written)
│   │   ├── mod.rs
│   │   ├── backend.rs           # Auth backend trait
│   │   ├── static_key.rs        # Static key auth (WireGuard-style)
│   │   ├── token.rs             # Token-based auth (HMAC-signed JWT-like)
│   │   └── certificate.rs       # X.509 certificate auth
│   │
│   ├── config/                  # Configuration
│   │   ├── mod.rs
│   │   ├── parser.rs            # TOML-like config parser (hand-written)
│   │   ├── types.rs             # Config structs
│   │   └── profiles/            # Stealth profiles
│   │       ├── mod.rs           # Profile loader (compiled-in + runtime override)
│   │       ├── chrome_131.rs    # Compiled-in Chrome 131 fingerprint
│   │       ├── firefox_130.rs   # Compiled-in Firefox 130 fingerprint
│   │       └── safari_18.rs     # Compiled-in Safari 18 fingerprint
│   │
│   └── util/                    # Shared utilities
│       ├── mod.rs
│       ├── buf.rs               # Zero-copy buffer pool
│       ├── ring_buf.rs          # Lock-free ring buffer
│       ├── alloc.rs             # Arena / bump allocator
│       ├── endian.rs            # Byte order helpers
│       └── log.rs               # Minimal logging (stderr)
│
├── Cargo.toml                   # Minimal curated dependencies (~10 direct)
├── DEPS.md                      # Dependency justification document
├── build.rs                     # CPU feature detection
└── README.md
```

---

## 4. Core Subsystem Specifications

### 4.1 Custom Async Runtime (`runtime/`)

No tokio. No async-std. No mio. Direct kernel interface.

**Reactor backends (compile-time selected):**

| Platform      | Backend     | Syscall Interface  |
|---------------|-------------|--------------------|
| Linux ≥ 5.11  | `io_uring`  | `libc` wrappers    |
| Linux < 5.11  | `epoll`     | `libc` wrappers    |
| macOS         | `kqueue`    | `libc` wrappers    |
| FreeBSD       | `kqueue`    | `libc` wrappers    |
| Windows       | `IOCP`      | `windows-sys` crate|

**Design:**

- Syscalls via `libc` crate for portability (no raw asm per-platform)
- Single-threaded event loop by default (1 core = 1 reactor)
- Optional work-stealing multi-threaded executor for server mode
- Timer wheel (hierarchical, 4 levels) for connection timeouts, keepalive, rekeying schedules
- Intrusive linked list task queue — zero heap allocation for task scheduling
- `Future` trait from `core::future` — no alloc needed for simple state machines
- Pinned task storage in pre-allocated arena

**Latency targets:**
- Event notification → task wake: < 1μs
- Full packet I/O round-trip through reactor: < 5μs on io_uring

### 4.2 Cryptography (`crypto/`)

Thin wrappers over audited, minimal crates. No hand-rolled primitives.

#### 4.2.1 Selected Crates

| Primitive            | Crate                     | Why this one                                        |
|----------------------|---------------------------|-----------------------------------------------------|
| ChaCha20-Poly1305    | `chacha20poly1305`        | RustCrypto, audited, SIMD-accelerated, `#[no_std]`  |
| AES-256-GCM          | `aes-gcm`                 | RustCrypto, AES-NI + CLMUL intrinsics, `#[no_std]`  |
| X25519               | `x25519-dalek`            | Widely audited, constant-time, `#[no_std]`           |
| Ed25519              | `ed25519-dalek`           | Same ecosystem as x25519, batch verify support       |
| ML-KEM-768           | `ml-kem`                  | RustCrypto, FIPS 203 compliant, `#[no_std]`          |
| BLAKE3               | `blake3`                  | Official crate, SIMD, ≥ 2 GB/s, `#[no_std]`         |
| CSPRNG               | `rand_core` + `rand_chacha` | OsRng + ChaCha20Rng DRBG, industry standard       |
| Key zeroization      | `zeroize`                 | Compiler-fence backed, derive macro, ~0 cost         |
| Constant-time ops    | `subtle`                  | RustCrypto standard, `Choice` / `ConstantTimeEq`    |

All RustCrypto crates share the same trait ecosystem (`aead`, `cipher`, `digest`), minimizing glue code.

**AEAD selection at handshake:**
- Client advertises supported AEADs
- Server picks fastest mutually supported
- Runtime benchmark on first launch, result cached (ChaCha20 wins on non-AES-NI hardware, AES-GCM wins with hardware support)

#### 4.2.2 Post-Quantum Hybrid Key Exchange

Apate uses a **hybrid key exchange** combining classical X25519 with ML-KEM-768 (FIPS 203). Both must be broken for the session to be compromised.

```
Hybrid SS = HKDF-BLAKE3(X25519_SS || ML-KEM_SS)
```

- **Why ML-KEM-768:** NIST standardized (FIPS 203), 128-bit post-quantum security, ciphertext ~1088 bytes (fits in 1 UDP packet with framing)
- **Why hybrid:** X25519 is battle-tested; ML-KEM is new. Hybrid ensures security even if one primitive is broken.
- **Wire impact:** Handshake grows by ~2.2 KB (ML-KEM public key + ciphertext). Acceptable for 1-RTT handshake on any modern link.
- **Performance:** ML-KEM encapsulate/decapsulate < 100μs — negligible vs network RTT
- **Config option:** `crypto.post_quantum = true` (default) — can be disabled for constrained environments

#### 4.2.2 HKDF (hand-written, ~50 LOC)

- Extract-then-Expand paradigm, built on top of `blake3`
- Simple enough to write ourselves, avoids pulling in the full `hkdf` crate + `hmac` dependency chain
- Used for deriving traffic keys, IVs, sub-keys from handshake output

#### 4.2.3 Crypto Wrapper Design

```rust
/// Unified AEAD interface — dispatches to ChaCha20-Poly1305 or AES-256-GCM
pub enum Cipher {
    ChaCha(chacha20poly1305::ChaCha20Poly1305),
    AesGcm(aes_gcm::Aes256Gcm),
}

impl Cipher {
    /// Encrypt in-place within the buffer (zero-copy)
    pub fn seal_in_place(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), CryptoError>;

    /// Decrypt in-place
    pub fn open_in_place(
        &self,
        nonce: &[u8; 12],
        aad: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), CryptoError>;
}
```

#### 4.2.4 Security Invariants

- **All crates selected are constant-time** by design (verified via `subtle` crate usage)
- **Zeroization:** all key material wrapped in `Zeroizing<T>` from `zeroize` crate
- **No custom crypto primitives** — only composition (Noise handshake, HKDF, nonce management)
- **Side-channel testing:** CI includes dudect-style timing tests on our composition layer
- **`cargo audit`** runs in CI, blocks release on any advisory

### 4.3 Noise Protocol Handshake (`noise/`)

Pattern: **Noise_IK** (client knows server's static public key)

```
← s                           (server static key, pre-distributed)
...
→ e, es, s, ss                (client sends ephemeral + encrypted static)
← e, ee, se                   (server responds with ephemeral)
```

**Why IK:**
- 1-RTT handshake (lowest possible for mutual auth)
- Client identity hidden from passive observers
- Server identity verified in first message
- Perfect forward secrecy via ephemeral keys

**Handshake latency budget:**

| Step                    | Target     |
|-------------------------|------------|
| Key generation (X25519) | < 50μs     |
| ML-KEM encapsulate      | < 100μs    |
| DH operations (×3)      | < 150μs    |
| AEAD encrypt/decrypt    | < 5μs      |
| HKDF combine (X25519+KEM)| < 5μs    |
| **Total 1-RTT**         | **< 400μs**|

**0-RTT Reconnection:**
- After initial handshake, both sides cache a "resumption PSK"
- Subsequent connections use Noise_IKpsk2 — client sends encrypted data in first message
- PSK rotated every N connections or T time
- 0-RTT data is replay-vulnerable — only used for non-critical initial packets (e.g., keepalive, padding)
- Critical data waits for full handshake confirmation

**Key Rotation (post-handshake):**
- Symmetric ratchet: HKDF(current_key, "ratchet") → new key every 60 seconds or 1 GB
- Direction-separated keys (client→server, server→client)
- Nonce: 64-bit counter, connection torn down if nonce space approaches exhaustion (2^62)

### 4.4 Transport Layer (`transport/`)

A custom lightweight multiplexed transport with **three wire modes**, selectable at connection time.

#### 4.4.1 Transport Modes

| Mode           | Wire Format                  | Port | When to Use                              |
|----------------|------------------------------|------|------------------------------------------|
| `tls13` (default) | Apate frames inside TLS 1.3 Application Data records over UDP | 443  | Most networks; looks like QUIC/HTTP3     |
| `quic`         | Apate frames disguised as QUIC Initial + Short Header packets | 443  | Networks that specifically whitelist QUIC |
| `tcp`          | Apate frames inside TLS 1.3 records over TCP                 | 443  | Restrictive networks that block all UDP  |

**Auto-negotiation:**
1. Client tries `tls13` (UDP) first
2. If UDP/443 is blocked (no response after 3s), falls back to `tcp`
3. User can force a specific mode via config: `transport.mode = "tcp"`

**TCP mode caveats:**
- Head-of-line blocking is inherent — a single lost TCP segment stalls all tunnel traffic
- Mitigation: TCP_NODELAY, aggressive keep-alive, application-level framing to detect stuck connections
- FEC is disabled in TCP mode (TCP already handles retransmission)
- Expected ~15-30% latency increase vs UDP modes under loss

**QUIC camouflage mode:**
- Mimics QUIC v1 (RFC 9000) wire format: Initial packets with fake Connection IDs, Short Header packets for data
- QUIC version field set to 0x00000001
- Does NOT implement real QUIC — only the packet header structure is mimicked
- Actual reliability/congestion handled by our own transport underneath

#### 4.4.2 Why Not Real QUIC

- Full QUIC (RFC 9000) is ~100K LOC — too large for zero-dep
- QUIC carries HTTP/3 baggage we don't need
- QUIC's loss recovery is designed for web, not bulk VPN tunnel traffic
- We want full control over packet structure for DPI evasion
- QUIC fingerprinting is a known DPI vector

#### 4.4.3 Frame Format

All frames are wrapped in the stealth layer (§4.5) before hitting the wire.

**Inner frame (plaintext before encryption):**

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|  Type (4) | Flags (4) |         Stream ID (16)               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Sequence Number (32)                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|         Payload Length (16)       |      FEC Group (8)        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                              |
|                      Payload (variable)                      |
|                                                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Frame types:**

| Type | Name       | Purpose                        |
|------|------------|--------------------------------|
| 0x0  | DATA       | Tunnel payload (IP packets)    |
| 0x1  | ACK        | Acknowledgment                 |
| 0x2  | FEC        | Forward error correction parity|
| 0x3  | PING       | Keepalive                      |
| 0x4  | REKEY      | Key rotation signal            |
| 0x5  | MIGRATE    | Connection migration           |
| 0x6  | CLOSE      | Graceful shutdown              |
| 0x7  | PAD        | Padding-only (stealth)         |

**Flags:**
- `FIN` (0x01): Last frame for this stream
- `PRIORITY` (0x02): High-priority frame (expedited processing)
- `COMPRESSED` (0x04): Payload is LZ4-compressed (optional)
- `FEC_PROTECTED` (0x08): This frame is part of an FEC group

#### 4.4.4 Multiplexed Streams

- Stream 0: Control channel (rekey, migrate, close)
- Stream 1: Primary tunnel data
- Streams 2+: Reserved for future use (split routing, parallel tunnels)
- Lightweight — stream is just a 16-bit ID prefix, no per-stream flow control (VPN doesn't need it)

#### 4.4.5 Congestion Control — BBR-Inspired

- Probes bandwidth and RTT continuously
- Pacing-based (not window-based) — smoother send rate
- Four states: STARTUP → DRAIN → PROBE_BW → PROBE_RTT
- Optimized for VPN: we control both endpoints, so we can be more aggressive than web BBR
- Pacing enforced via timer wheel — packets released at calculated intervals
- ECN support for cooperative congestion signaling

#### 4.4.6 Loss Detection & Recovery

**Hybrid approach:**
1. **ACK-based:** Selective ACKs (SACK) with bitmask for received packets
2. **Timer-based:** Tail Loss Probe (TLP) after 2× smoothed RTT
3. **FEC-based:** For FEC-protected groups, lost packets reconstructed without retransmit

**ACK frame format:**

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                 Largest Acknowledged (32)                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    ACK Delay (16)          | Block Count (8)  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   SACK Bitmask (variable)                     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

#### 4.4.7 Forward Error Correction (FEC)

- Reed-Solomon GF(2^8) with configurable k/n ratio
- Default: k=8 data packets, n=10 total (2 parity) → tolerates 20% loss without retransmit
- Adaptive: FEC ratio adjusted based on observed loss rate
  - < 1% loss: FEC disabled (pure retransmit)
  - 1–5% loss: k=8, n=9 (1 parity)
  - 5–15% loss: k=8, n=10 (2 parity)
  - > 15% loss: k=4, n=6 (2 parity, smaller groups)
- FEC encoding/decoding: Galois field arithmetic tables, SIMD-accelerated matrix operations

**Latency impact:** FEC eliminates retransmit round-trips for random loss. On a 100ms RTT link with 5% loss, FEC reduces p99 latency from ~300ms to ~105ms.

#### 4.4.8 Connection Migration

- Connection identified by a 128-bit connection ID (not by IP:port)
- When client's IP changes (e.g., Wi-Fi → cellular), it sends a MIGRATE frame with proof of possession
- Proof: HMAC(migration_key, new_ip || new_port || timestamp)
- Server validates and updates the peer address — no re-handshake needed
- Seamless for the tunnel — zero downtime on network switch

#### 4.4.9 LZ4 Payload Compression

Optional per-frame compression via LZ4 (hand-written, ~400 LOC — the algorithm is simple enough).

- **When enabled:** `COMPRESSED` flag (0x04) set in frame header
- **Negotiation:** both sides advertise support in handshake; compression only used if both agree
- **Heuristic:** only compress if payload > 128 bytes AND estimated compression ratio > 10%. Skip already-encrypted or high-entropy payloads (e.g., HTTPS inside tunnel — already TLS-encrypted, won't compress)
- **Why LZ4:** fastest decompressor available (~4 GB/s), < 1μs overhead per typical packet, minimal latency impact
- **Implementation:** LZ4 block format only (no framing), hand-written to avoid crate dependency. LZ4 is simple enough: literal/match sequences with 4-byte hash table.
- **Config:** `transport.compression = "auto"` (default) | `"on"` | `"off"`. Auto skips compression when entropy is high.

### 4.5 Stealth / DPI Evasion (`stealth/`)

This is Apate's defining feature. Three layers of evasion:

#### 4.5.1 Layer 1 — TLS 1.3 Record Camouflage

All wire packets are wrapped in TLS 1.3 Application Data record format:

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Content Type (0x17) | Version (0x0303) | Length   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                   |
|            Encrypted Apate Frame                  |
|            (looks like TLS ciphertext)            |
|                                                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| AEAD Tag (16 bytes)                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Initial handshake mimicry:**
1. Client sends a **real-looking TLS 1.3 ClientHello** (mimicking Chrome/Firefox fingerprint)
   - Correct cipher suite ordering, extensions, supported groups
   - SNI set to configured cover domain (e.g., "cdn.example.com")
   - ECH (Encrypted Client Hello) support when available
   - Session ID, compression methods, padding — all matching real browser
2. Server responds with a **real-looking ServerHello + encrypted extensions**
3. Underneath, the Noise IK handshake is embedded within these messages
4. After handshake, all data flows as TLS 1.3 Application Data records

**Key insight:** DPI boxes check the TLS handshake for anomalies. By perfectly mimicking a real browser's ClientHello, we pass structural checks. The actual crypto (Noise) happens inside the payload, invisible to inspectors.

#### 4.5.2 Layer 2 — Traffic Shaping

**Packet length distribution:**
- Real HTTPS traffic has characteristic packet size distributions
- Apate pads packets to match a target distribution profile
- Padding added as random bytes before encryption (indistinguishable from ciphertext)

**Inter-packet timing:**
- DPI uses timing analysis to fingerprint tunnels (VPN traffic is "too regular")
- Apate adds calibrated jitter to inter-packet gaps
- Jitter drawn from distribution fitted to real HTTPS traffic patterns
- Maximum added latency: configurable, default 2ms p99

**Dummy traffic:**
- Optional: send PAD frames during idle periods to prevent traffic analysis
- Configurable rate (e.g., 1 KB/s baseline during idle)
- Prevents "connection goes silent → user inactive → VPN tunnel" inference

#### 4.5.3 Pluggable Stealth Profiles

Stealth profiles define browser fingerprints and traffic shaping parameters. **Compiled-in defaults with runtime override.**

**Profile structure (TOML when loaded from file):**

```toml
[fingerprint]
name = "chrome_131"
cipher_suites = [0x1301, 0x1302, 0x1303, 0xc02b, 0xc02f]
extensions_order = ["server_name", "ec_point_formats", "supported_groups", ...]
supported_groups = ["x25519", "secp256r1", "secp384r1"]
signature_algorithms = ["ecdsa_secp256r1_sha256", "rsa_pss_rsae_sha256", ...]
alpn = ["h2", "http/1.1"]

[shaping]
type = "web-browsing"                     # youtube-streaming | web-browsing | file-download
packet_sizes = { mean = 850, stddev = 400, min = 40, max = 1400 }
inter_packet_ms = { mean = 12, stddev = 8 }
idle_rate_kbps = 1
```

**Loading order:**
1. Check `stealth.profile_path` in config → load custom TOML profile
2. If not set, use `stealth.profile` name → match compiled-in profile
3. Compiled-in profiles: `chrome_131`, `firefox_130`, `safari_18`

**Runtime hot-reload:**
- SIGHUP to the process re-reads profile from disk (if `profile_path` is set)
- Zero downtime — new connections use new profile, existing connections unchanged

#### 4.5.4 Layer 3 — Active Probing Defense

When censors detect a suspicious server, they "actively probe" it — connect and see if it behaves like a real web server.

**Apate server's defense:**

```
                    ┌─────────────┐
  Incoming          │   Detect    │
  Connection ──────→│  Auth Cookie│
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              │                         │
        Valid Cookie              No/Invalid Cookie
              │                         │
              ▼                         ▼
     ┌────────────────┐       ┌─────────────────┐
     │  Apate Tunnel  │       │  Web Facade      │
     │  (VPN mode)    │       │  (HTTP server)   │
     └────────────────┘       │  Serves real     │
                              │  website content │
                              └─────────────────┘
```

- **Auth Cookie:** A pre-shared secret embedded in the ClientHello (e.g., in session_id field or a custom extension). Without it, the server is a normal web server.
- **Web Facade:** Serves a real, configured website (static HTML, or reverse proxies to a real site). Returns valid TLS certificates (via embedded ACME/Let's Encrypt client or pre-provisioned).
- **Behavioral mimicry:** Response timing, header ordering, and TLS behavior matches the configured web server profile (nginx, Apache, Caddy).

### 4.6 TUN Interface (`tunnel/`)

Direct kernel interaction, no crate dependencies.

**Linux:**
```rust
// Open /dev/net/tun
let fd = syscall!(SYS_open, b"/dev/net/tun\0", O_RDWR);

// Configure via ioctl
let mut ifr: ifreq = zeroed();
ifr.ifr_name = *b"apate0\0\0\0\0\0\0\0\0\0\0";
ifr.ifr_flags = IFF_TUN | IFF_NO_PI;
syscall!(SYS_ioctl, fd, TUNSETIFF, &ifr);

// Set IP address via netlink
netlink_set_addr(ifr.ifr_name, "10.0.0.2/24");
netlink_set_up(ifr.ifr_name);
```

**macOS:**
```rust
// Use utun kernel control socket
let fd = syscall!(SYS_socket, PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL);
let ctl = ctl_info { ctl_id: 0, ctl_name: *b"com.apple.net.utun_control\0..." };
syscall!(SYS_ioctl, fd, CTLIOCGINFO, &ctl);
// connect() to claim utunN
```

**Packet I/O:**
- Packets read/written to TUN fd directly
- io_uring for Linux (register TUN fd as fixed file), epoll fallback
- macOS: kqueue on utun fd
- Zero-copy: read directly into pre-allocated packet buffer pool

### 4.7 Routing (`routing/`)

**LPC Trie (Longest Prefix Compression):**
- Memory-efficient IP prefix lookup
- O(W) lookup where W = address bit width (32 for IPv4, 128 for IPv6)
- Supports split tunneling: configurable routes for which traffic goes through tunnel

**DNS leak prevention:**
- Intercept DNS queries (UDP/53, TCP/53, DoH/443) at TUN level
- Route all DNS through tunnel

**Built-in DNS-over-HTTPS resolver (`routing/doh.rs`):**
- Minimal HTTP/1.1 client (hand-written, ~300 LOC — only needs POST to `/dns-query`)
- Sends DNS wire-format queries over the already-encrypted tunnel to configured upstream (e.g., `https://1.1.1.1/dns-query`)
- Reuses tunnel's TLS camouflage — DoH traffic is indistinguishable from normal HTTPS inside the tunnel
- Connection pooling: single persistent HTTP connection to upstream resolver
- Fallback: if DoH fails, forward plain DNS through tunnel (still leak-proof since it's inside the tunnel)
- Config: `dns.mode = "doh"` (default) | `"plain"` | `"doh-fallback-plain"`
- Configurable upstreams: `dns.doh_upstreams = ["https://1.1.1.1/dns-query", "https://dns.google/dns-query"]`

### 4.8 Buffer Management (`util/`)

Zero-copy is critical for latency.

**Buffer pool:**
- Pre-allocated arena of MTU-sized buffers (default: 4096 × 1500 bytes ≈ 6 MB)
- Lock-free free-list for multi-threaded access
- Packet travels from TUN read → encrypt → pad → send without memcpy
- Buffer layout designed for in-place encryption:

```
┌──────────────┬──────────────────────┬──────────┬──────────┐
│  Headroom    │     IP Packet        │  Padding │ AEAD Tag │
│  (TLS hdr)  │     (from TUN)       │          │          │
│  5 bytes     │     variable         │ variable │ 16 bytes │
└──────────────┴──────────────────────┴──────────┴──────────┘
```

Headroom and tag space pre-allocated — encryption operates in-place on the IP packet region, then TLS header prepended and tag appended without copying.

---

## 5. Wire Protocol Specification

### 5.1 Connection Establishment

```
Client                                              Server
  │                                                    │
  │─── [1] TLS ClientHello (with embedded cookie) ────→│
  │    (Contains Noise IK: e, es, s, ss)               │
  │                                                    │
  │←── [2] TLS ServerHello + EncryptedExtensions ──────│
  │    (Contains Noise IK: e, ee, se)                  │
  │                                                    │
  │    ─── Noise transport keys established ───        │
  │                                                    │
  │─── [3] TLS AppData (encrypted DATA frame) ────────→│
  │←── [4] TLS AppData (encrypted DATA frame) ────────│
  │                                                    │
  │    ═══ Bidirectional tunnel active ═══             │
  │                                                    │
```

Total: **1 RTT** to first data.
With 0-RTT PSK resumption: **0 RTT** (data in first packet).

### 5.2 Packet Pipeline (Client → Server)

```
IP Packet from TUN
       │
       ▼
┌──────────────┐
│ Route lookup │ → bypass? → direct send (not tunneled)
└──────┬───────┘
       │ tunnel
       ▼
┌──────────────┐
│ FEC encode   │ → add to FEC group, generate parity if group full
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Frame build  │ → prepend frame header (type, stream, seq, length)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ AEAD encrypt │ → ChaCha20-Poly1305 in-place, append 16-byte tag
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Pad + Shape  │ → add padding to match target size distribution
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ TLS wrap     │ → prepend 5-byte TLS Application Data header
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Pacing queue │ → release at BBR-calculated rate + jitter
└──────┬───────┘
       │
       ▼
  UDP sendmsg()
```

### 5.3 Steady-State Latency Budget (per packet)

| Step              | Target      |
|-------------------|-------------|
| TUN read          | 1μs         |
| Route lookup      | 0.1μs       |
| FEC encode        | 2μs         |
| Frame build       | 0.1μs       |
| AEAD encrypt      | 1μs (1400B) |
| Pad + shape       | 0.1μs       |
| TLS wrap          | 0.05μs      |
| Pacing delay      | 0-2ms*      |
| UDP send          | 1μs         |
| **Total overhead**| **< 10μs**  |

*Pacing delay is network-dependent, not protocol overhead.

---

## 6. Configuration

Hand-written TOML-subset parser (full TOML is overkill).

### 6.1 Client Configuration

```toml
[client]
server = "vpn.example.com:443"
server_pubkey = "base64_encoded_ed25519_public_key"
tunnel_ip = "10.0.0.2/24"
dns = ["1.1.1.1", "8.8.8.8"]
mtu = 1400

[dns]
mode = "doh"                      # doh | plain | doh-fallback-plain
doh_upstreams = ["https://1.1.1.1/dns-query", "https://dns.google/dns-query"]

[auth]
method = "static_key"              # static_key | token | certificate
private_key = "/etc/apate/client.key"
# token = "eyJhbGciOi..."         # for token auth
# certificate = "/etc/apate/client.crt"  # for cert auth

[stealth]
mode = "tls13"                    # tls13 | quic | raw (no camouflage)
sni = "cdn.example.com"
profile = "chrome_131"            # compiled-in: chrome_131 | firefox_130 | safari_18
profile_path = ""                 # optional: path to custom profile TOML (overrides above)
timing_jitter_ms = 2
idle_traffic = true

[transport]
mode = "auto"                     # auto | udp | tcp (auto = UDP first, TCP fallback)
fallback_timeout_secs = 3         # seconds before falling back from UDP to TCP
fec = "adaptive"                  # off | adaptive | fixed (disabled in TCP mode)
fec_ratio = "8:10"                # k:n for fixed mode
congestion = "bbr"                # bbr | none (trusted LAN)
compression = "auto"              # auto | on | off (auto skips high-entropy)

[crypto]
aead = "auto"                     # auto | chacha20-poly1305 | aes-256-gcm
post_quantum = true               # hybrid X25519 + ML-KEM-768
rekey_interval_secs = 60
rekey_interval_bytes = 1073741824  # 1 GB

[routing]
mode = "full"                     # full | split
bypass = ["192.168.0.0/16", "10.0.0.0/8"]
```

### 6.2 Server Configuration

```toml
[server]
listen = "0.0.0.0:443"
private_key = "/etc/apate/server.key"
tunnel_subnet = "10.0.0.0/24"
max_clients = 256

[stealth]
mode = "tls13"                              # tls13 | quic | both
facade_backend = "https://example.com"      # reverse proxy target for non-VPN connections
tls_cert = "/etc/apate/cert.pem"
tls_key = "/etc/apate/cert.key"
profile = "chrome_131"                      # or profile_path for custom

[auth]
methods = ["static_key", "token", "certificate"]  # enabled auth backends
allowed_keys = "/etc/apate/authorized_keys"        # for static_key
cookie_secret = "base64_encoded_32_bytes"
token_secret = "base64_encoded_32_bytes"           # HMAC key for token verification
ca_cert = "/etc/apate/ca.pem"                      # for certificate auth

[transport]
listen_tcp = true                # also listen on TCP/443 for fallback clients
fec = "adaptive"
congestion = "bbr"

[crypto]
post_quantum = true              # require hybrid KX from clients
```

---

## 7. Security Model

### 7.1 Threat Model

| Threat                     | Mitigation                                      |
|----------------------------|--------------------------------------------------|
| Passive DPI                | TLS 1.3 camouflage, traffic shaping              |
| Active probing             | Web facade, cookie-based client detection         |
| Statistical analysis       | Packet length padding, timing jitter, dummy traffic|
| Replay attacks             | Nonce counters, 0-RTT limited to non-critical data|
| Key compromise             | PFS via ephemeral keys, 60s rekeying              |
| Side-channel (timing)      | Constant-time crypto, no secret-dependent branches|
| Supply chain               | Minimal deps (~10 direct), `cargo audit` + `cargo deny` in CI |
| Connection correlation     | Connection migration, no fixed identifiers        |
| DNS leaks                  | All DNS forced through tunnel                     |

### 7.2 Key Hierarchy

```
Server Static Key (Ed25519)
    │
    └── Verified by client (pinned or TOFU)

Handshake:
    Client Ephemeral (X25519) ─┐
    Server Ephemeral (X25519) ─┤
    Client Static (X25519) ────┤──→ Noise IK ──→ Handshake Key
    Server Static (X25519) ────┘         │
                                         │
                                    HKDF-BLAKE3
                                         │
                               ┌─────────┴─────────┐
                               ▼                     ▼
                     Client→Server Key      Server→Client Key
                               │                     │
                          ┌────┴────┐           ┌────┴────┐
                          ▼         ▼           ▼         ▼
                       AES-GCM  ChaCha20    AES-GCM  ChaCha20
                       (data)   (data)      (data)   (data)
                               │                     │
                        Ratchet every 60s / 1GB
```

---

## 8. Dependency Manifest

### 8.1 Cargo.toml (direct dependencies)

```toml
[dependencies]
# Crypto — audited, #[no_std] compatible
chacha20poly1305 = { version = "0.10", default-features = false, features = ["alloc"] }
aes-gcm          = { version = "0.10", default-features = false, features = ["alloc"] }
x25519-dalek     = { version = "2", default-features = false, features = ["static_secrets"] }
ed25519-dalek    = { version = "2", default-features = false, features = ["fast"] }
ml-kem           = { version = "0.2", default-features = false, features = ["ml-kem-768"] }
blake3           = { version = "1", default-features = false }
rand_core        = { version = "0.6", default-features = false, features = ["getrandom"] }
rand_chacha      = { version = "0.3", default-features = false }
zeroize          = { version = "1", default-features = false, features = ["derive"] }
subtle           = { version = "2", default-features = false }

# Platform
libc             = { version = "0.2", default-features = false }

[target.'cfg(windows)'.dependencies]
windows-sys      = { version = "0.59", features = ["Win32_System_IO", "Win32_Networking_WinSock"] }

[dev-dependencies]
criterion = "0.5"    # benchmarks

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### 8.2 Dependency Justification (`DEPS.md`)

| Crate               | Direct Deps | Justification                                               |
|----------------------|-------------|--------------------------------------------------------------|
| `chacha20poly1305`   | ~4          | Audited AEAD, SIMD-accelerated, constant-time                |
| `aes-gcm`           | ~4          | Audited AEAD, AES-NI hardware acceleration                   |
| `x25519-dalek`      | ~3          | Formally verified field arithmetic (fiat-crypto backend)     |
| `ed25519-dalek`     | ~3          | Same ecosystem, batch verification, widely deployed          |
| `ml-kem`            | ~3          | RustCrypto, FIPS 203 compliant, post-quantum KEM             |
| `blake3`            | ~1          | Official implementation, ≥ 2 GB/s, used by WireGuard-rs too |
| `rand_core`         | ~1          | Trait definitions + OsRng, minimal                           |
| `rand_chacha`       | ~1          | ChaCha20-based DRBG, deterministic for testing               |
| `zeroize`           | 0           | Zero-cost key zeroization, derive macro                      |
| `subtle`            | 0           | Constant-time primitives (`Choice`, `ConstantTimeEq`)        |
| `libc`              | 0           | Syscall types/constants, no runtime cost                     |
| `windows-sys`       | 0           | Windows API bindings (IOCP, WinSock), cfg(windows) only      |

**Estimated totals:** ~12 direct, ~42 transitive (within 50 limit)

### 8.3 What We Explicitly Do NOT Depend On

| Crate/Category       | Reason                                                     |
|----------------------|-------------------------------------------------------------|
| `tokio` / `async-std`| 200+ transitive deps, massive binary, unneeded features     |
| `quinn` / `quic`     | Full QUIC is overkill, we need custom transport             |
| `rustls` / `openssl` | We don't do real TLS, just mimic the wire format            |
| `hyper` / `reqwest`  | HTTP framework too heavy, facade is hand-written            |
| `serde` / `toml`     | Config parser is < 300 LOC, not worth 30+ transitive deps   |
| `tun` / `tun-tap`    | Thin ioctl wrapper, platform-specific, < 200 LOC each       |
| `ring`               | Duplicates RustCrypto; pick one ecosystem, not both          |
| `clap`               | CLI is simple, hand-written arg parser suffices              |

---

## 9. Build & Platform Support

### 9.1 Build

```bash
# Standard build (uses detected CPU features)
cargo build --release

# Minimal build (no SIMD, max compatibility)
cargo build --release --features no-simd

# Static binary (fully self-contained)
RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-musl

# Cross-compile for ARM64 (routers, embedded)
cargo build --release --target aarch64-unknown-linux-musl
```

### 9.2 Binary Size Target

| Configuration         | Target Size   |
|-----------------------|---------------|
| Release (stripped)    | < 2 MB        |
| Release + LTO         | < 1.5 MB      |
| `#[no_std]` embedded | < 500 KB      |

### 9.3 Platform Matrix

| Platform          | Status  | Reactor   | TUN             |
|-------------------|---------|-----------|-----------------|
| Linux x86_64      | v1.0    | io_uring  | /dev/net/tun    |
| Linux aarch64     | v1.0    | io_uring  | /dev/net/tun    |
| Linux (old kernel)| v1.0    | epoll     | /dev/net/tun    |
| Windows x86_64    | v1.0    | IOCP      | WinTUN          |
| macOS x86_64      | v1.0    | kqueue    | utun            |
| macOS aarch64     | v1.0    | kqueue    | utun            |
| FreeBSD           | v1.1    | kqueue    | /dev/tun        |
| OpenWrt (MIPS)    | v1.1    | epoll     | /dev/net/tun    |

---

## 10. Performance Targets

### 10.1 Throughput

| Scenario                    | Target         |
|-----------------------------|----------------|
| Single core, 1 Gbps link    | > 950 Mbps     |
| Single core, 10 Gbps link   | > 5 Gbps       |
| Multi-core, 10 Gbps link    | > 9 Gbps       |

### 10.2 Latency Overhead

| Metric                  | Target      |
|-------------------------|-------------|
| Per-packet overhead     | < 10μs      |
| Handshake (1-RTT)       | < 400μs     |
| Reconnect (0-RTT)       | < 50μs      |
| Rekey (seamless)        | 0 added     |

### 10.3 Memory

| Component               | Budget      |
|--------------------------|-------------|
| Buffer pool              | 6 MB        |
| Routing table (10K rules)| < 1 MB      |
| Crypto state per conn    | < 1 KB      |
| Total server (256 clients)| < 32 MB    |

---

## 11. Development Phases

### Phase 1 — Foundation (Weeks 1–3)

- [ ] `Cargo.toml` — add curated crypto dependencies (incl. `ml-kem`), configure `cargo deny`
- [ ] `DEPS.md` — document and justify every dependency
- [ ] `runtime/reactor.rs` — epoll reactor via `libc` (simpler, get things working first)
- [ ] `runtime/executor.rs` — single-threaded task executor
- [ ] `runtime/waker.rs` — waker implementation
- [ ] `crypto/aead.rs` — `Cipher` enum wrapping `chacha20poly1305` + `aes-gcm`, in-place API
- [ ] `crypto/kx.rs` — X25519 + ML-KEM-768 hybrid key exchange wrapper
- [ ] `crypto/sign.rs` — Ed25519 wrapper over `ed25519-dalek`
- [ ] `crypto/kdf.rs` — BLAKE3-HKDF (~50 LOC, uses `blake3` crate)
- [ ] `crypto/rng.rs` — `OsRng` + `ChaCha20Rng` via `rand_core`/`rand_chacha`
- [ ] Crypto integration tests: test vectors for AEAD/KDF/hybrid-KX compositions

### Phase 2 — Protocol Core (Weeks 4–7)

- [ ] `noise/` — Noise IK handshake extended with hybrid X25519+ML-KEM
- [ ] `transport/frame.rs` — frame encode/decode
- [ ] `transport/connection.rs` — connection state machine (UDP mode first)
- [ ] `transport/loss.rs` — ACK processing, SACK, retransmit
- [ ] `transport/congestion.rs` — BBR implementation
- [ ] `transport/pacing.rs` — send pacing
- [ ] `transport/lz4.rs` — LZ4 block compression/decompression (hand-written, ~400 LOC)
- [ ] `auth/backend.rs` — auth backend trait definition
- [ ] `auth/static_key.rs` — static key auth (WireGuard-style, default)
- [ ] `tunnel/tun_linux.rs` — TUN device open, configure, read/write (via `libc` ioctl)
- [ ] `routing/table.rs` — LPC trie
- [ ] `routing/dns.rs` — DNS interception
- [ ] `routing/doh.rs` — built-in DNS-over-HTTPS stub resolver (hand-written HTTP/1.1 POST client)
- [ ] Integration: TUN ↔ transport ↔ crypto ↔ UDP — basic tunnel works

### Phase 3 — Stealth & Transport Modes (Weeks 8–12)

- [ ] `stealth/tls_camouflage.rs` — TLS 1.3 record wrapping (UDP mode)
- [ ] `stealth/client_hello.rs` — Chrome-like ClientHello generation
- [ ] `stealth/server_hello.rs` — matching ServerHello
- [ ] `stealth/quic_camouflage.rs` — QUIC v1 packet header mimicry (UDP/443)
- [ ] `stealth/padding.rs` — packet size distribution matching
- [ ] `stealth/timing.rs` — inter-packet jitter
- [ ] `stealth/facade.rs` — HTTP web facade for probing defense
- [ ] `config/profiles/` — compiled-in stealth profiles (Chrome, Firefox, Safari)
- [ ] `config/profiles/mod.rs` — profile loader with runtime TOML override + SIGHUP hot-reload
- [ ] `transport/tcp.rs` — TCP transport mode (TLS 1.3 records over TCP/443)
- [ ] `transport/mode.rs` — auto-negotiation: UDP → TCP fallback with configurable timeout
- [ ] `config/parser.rs` — TOML subset parser
- [ ] End-to-end stealth tunnel test against DPI simulators (all 3 modes)

### Phase 4 — Auth, Platforms & Hardening (Weeks 13–18)

- [ ] `auth/token.rs` — HMAC-signed token auth
- [ ] `auth/certificate.rs` — X.509 certificate auth (hand-written ASN.1 parser, minimal)
- [ ] `transport/fec.rs` — Reed-Solomon FEC (disabled in TCP mode)
- [ ] `transport/migration.rs` — connection migration
- [ ] `runtime/reactor.rs` — io_uring backend (via `libc` io_uring syscalls)
- [ ] `runtime/iocp.rs` — Windows IOCP reactor (via `windows-sys`)
- [ ] `runtime/executor.rs` — multi-threaded work-stealing
- [ ] `tunnel/tun_windows.rs` — Windows WinTUN support (wintun.dll FFI)
- [ ] `tunnel/tun_macos.rs` — macOS utun support
- [ ] `cargo audit` + `cargo deny` CI pipeline
- [ ] Fuzzing: frame parser, handshake, config parser, profile loader, LZ4
- [ ] Performance benchmarks, memory profiling
- [ ] Documentation, man pages

---

## 12. Testing Strategy

### 12.1 Unit Tests

- Crypto wrappers: verify our composition layer (Cipher enum, HKDF, nonce management) with known test vectors
  - AEAD: RFC 8439 / NIST test vectors through our wrapper
  - HKDF: custom BLAKE3-HKDF test vectors (we implement this ourselves)
  - Nonce management: counter exhaustion, rekeying triggers
- Noise handshake: Noise Explorer test vectors
- Frame encode/decode: round-trip property tests
- LPC trie: correctness with known routing tables

### 12.2 Integration Tests

- Full handshake between client and server processes
- Tunnel data integrity: send known data through tunnel, verify on other side
- Connection migration: change client IP mid-stream, verify continuity
- FEC recovery: drop N% of packets, verify data integrity
- Rekey: verify seamless key rotation under load

### 12.3 DPI Evasion Tests

- **tlsfingerprint.io compatibility:** captured ClientHello matches target browser
- **Entropy analysis:** encrypted payload entropy ≈ TLS ciphertext entropy (close to 8.0 bits/byte)
- **Packet length distribution:** KL divergence from target profile < 0.01
- **Timing analysis:** inter-packet timing passes Anderson-Darling test against target distribution
- **Active probing:** connect without cookie → verify web facade serves real HTTP

### 12.4 Performance Tests

- `iperf3` through tunnel: measure throughput, jitter, latency
- `wrk` against web facade: verify it behaves under load
- Crypto benchmarks: cycles/byte for each primitive
- Memory profiling: verify no leaks, buffer pool efficiency

### 12.5 Security Tests

- Fuzzing: `cargo-fuzz` on frame parser, handshake, config parser
- Timing side-channels: `dudect` on all constant-time operations
- Key zeroization: verify via `/proc/[pid]/mem` inspection
- Nonce reuse detection: stress test nonce counter logic

---

## 13. Comparison

| Feature                | WireGuard | OpenVPN  | Apate        |
|------------------------|-----------|----------|--------------|
| Dependencies           | Kernel/Go | OpenSSL  | **Minimal (~10)**|
| Handshake RTT          | 1         | 2-3      | **1 (0 PSK)**|
| DPI resistance         | None      | Minimal  | **Full**     |
| Protocol obfuscation   | No        | Pluggable| **Built-in** |
| Active probing defense | No        | No       | **Yes**      |
| Traffic shaping        | No        | No       | **Yes**      |
| FEC                    | No        | No       | **Yes**      |
| Connection migration   | Partial   | No       | **Yes**      |
| Per-packet overhead    | ~10μs     | ~50μs    | **< 10μs**   |
| Binary size            | ~600KB    | ~2MB     | **< 1.5MB**  |
| Codebase               | ~4K LOC   | ~100K LOC| **~15K LOC** |

---

## 14. Resolved Design Decisions

| #  | Question                          | Decision                                                                 |
|----|-----------------------------------|--------------------------------------------------------------------------|
| 1  | QUIC fallback mode?               | **Yes.** Fake QUIC (UDP/443) supported as a stealth mode alongside TLS 1.3. |
| 2  | TCP mode for extreme environments?| **Yes.** TCP/443 fallback is mandatory. Auto-negotiation: UDP first, TCP after 3s timeout. FEC disabled in TCP mode. |
| 3  | Pluggable stealth profiles?       | **Both.** Compiled-in defaults (Chrome/Firefox/Safari) + runtime override via TOML file. SIGHUP hot-reload. |
| 4  | Client authentication?            | **Pluggable auth backend.** Three methods: static key (default), HMAC-signed tokens, X.509 certificates. Server enables any combination. |
| 5  | Post-quantum readiness?           | **v1.0.** Hybrid X25519 + ML-KEM-768 (FIPS 203) key exchange. Enabled by default, configurable off. |
| 6  | Windows support?                  | **v1.0.** WinTUN + IOCP reactor. All major desktop platforms ship in v1.0. |
| 7  | Payload compression?              | **v1.0.** Hand-written LZ4 block compression (~400 LOC). Auto mode skips high-entropy payloads. |
| 8  | DNS-over-HTTPS?                   | **v1.0.** Built-in DoH stub resolver, enabled by default. Hand-written minimal HTTP/1.1 POST client. |
