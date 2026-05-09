# Apate Architecture

This document describes the high-level architecture of the Apate stealth VPN
protocol implementation: the module decomposition, the data flow through the
packet processing pipeline, the connection lifecycle state machines, and the
cross-platform strategy.

---

## 1. Design Principles

Apate is built around four architectural patterns applied consistently across
all subsystems.

**Ports and Adapters** separates the authentication and platform layers from
the protocol core. `auth::AuthBackend` is a trait; `AuthCoordinator` dispatches
to registered backend implementations (`StaticKeyBackend`, `TokenBackend`,
`CertificateBackend`) without the protocol core having any knowledge of how
credentials are stored or verified. Tunnel adapters (`tun_linux`, `tun_macos`,
`tun_windows`, `tun_freebsd`) implement a common interface so the routing and
transport layers are OS-agnostic.

**State Machine** governs the connection lifecycle and the Noise handshake.
Both are implemented as strict enum-driven state machines where invalid
transitions return a typed error rather than panicking. No implicit state
mutation occurs outside of the `transition` method.

**Strategy** encapsulates transport selection behind the `TransportStrategy`
trait. The concrete transport (`UdpTls`, `TcpTls`, `QuicMask`) is determined
at runtime by `ModeNegotiator` and injected. Neither the tunnel layer nor the
application layer references a concrete transport type.

**Pipeline** structures packet processing as a linear sequence of stages.
Each stage is a pure function or a small stateful component that takes an input
and produces an output or an error. Side effects (I/O, timing) are isolated to
the edges of the pipeline.

---

## 2. Module Structure

```
src/
├── lib.rs              -- crate root; top-level error enum ApateError
├── main.rs             -- binary entry point; delegates to cli
├── cli/                -- command-line interface
│   ├── mod.rs
│   ├── args.rs         -- clap argument definitions
│   └── commands.rs     -- subcommand dispatch (client, server, keygen)
├── runtime/            -- async I/O runtime (no tokio dependency)
│   ├── mod.rs
│   ├── reactor.rs      -- FIFO event queue (ReactorEvent)
│   ├── executor.rs     -- task executor
│   ├── timer.rs        -- timer wheel
│   ├── waker.rs        -- waker implementation
│   └── backend/        -- OS-specific event backends
│       ├── mod.rs
│       ├── epoll.rs    -- Linux epoll(7)
│       ├── io_uring.rs -- Linux io_uring
│       ├── kqueue.rs   -- macOS/FreeBSD kqueue(2)
│       └── iocp.rs     -- Windows I/O completion ports
├── crypto/             -- cryptographic primitives
│   ├── mod.rs          -- CryptoError
│   ├── aead.rs         -- ChaCha20-Poly1305 and AES-GCM wrappers
│   ├── kx.rs           -- X25519 key exchange (derive_public_key, derive_shared_secret)
│   ├── sign.rs         -- Ed25519 sign / verify
│   ├── kdf.rs          -- BLAKE3-based key derivation
│   └── rng.rs          -- ChaCha20 CSPRNG wrapper
├── noise/              -- Noise protocol handshake
│   ├── mod.rs          -- SecurityError; re-exports HandshakeState, NoiseSession
│   ├── state.rs        -- HandshakeState enum; NoiseSession struct with transition()
│   ├── handshake.rs    -- HandshakeMachine; HandshakeMessage processing
│   ├── cipher_state.rs -- per-direction cipher state (key + nonce counter)
│   └── symmetric_state.rs -- transcript hash accumulation (mix_hash, mix_key)
├── transport/          -- wire framing, transport selection, reliability
│   ├── mod.rs          -- FrameError, TransportError; TransportStrategy trait
│   ├── frame.rs        -- Frame, FrameType, encode_frame, decode_frame
│   ├── mode.rs         -- TransportKind, ModeNegotiator, AttemptOutcome
│   ├── connection.rs   -- connection handle
│   ├── ack.rs          -- cumulative and selective ACK tracking
│   ├── loss.rs         -- loss detection
│   ├── congestion.rs   -- congestion window management
│   ├── pacing.rs       -- packet pacing
│   ├── fec.rs          -- FecController; FecMode (Disabled/SingleParity/DoubleParity)
│   ├── migration.rs    -- connection migration (MIGRATE frame handling)
│   ├── udp_tls.rs      -- UDP + DTLS transport implementation
│   ├── tcp_tls.rs      -- TCP + TLS transport implementation
│   └── quic_mask.rs    -- QUIC-masked transport implementation
├── stealth/            -- traffic camouflage and active-probe defense
│   ├── mod.rs          -- StealthRuntime; profile loading
│   ├── tls_camouflage.rs  -- TLS ClientHello/ServerHello mimicry
│   ├── quic_camouflage.rs -- QUIC Initial packet mimicry
│   ├── client_hello.rs -- ClientHello construction
│   ├── server_hello.rs -- ServerHello construction
│   ├── padding.rs      -- length-distribution padding
│   ├── timing.rs       -- inter-packet jitter injection
│   ├── entropy.rs      -- payload entropy normalization
│   └── facade.rs       -- probe gate: serve decoy response on auth failure
├── tunnel/             -- OS TUN adapter
│   ├── mod.rs
│   ├── packet.rs       -- IP packet parsing utilities
│   ├── tun_linux.rs    -- Linux TUN via /dev/net/tun + ioctl
│   ├── tun_macos.rs    -- macOS TUN via /dev/tun + ioctl
│   ├── tun_windows.rs  -- Windows TUN via WinTUN + windows-sys
│   └── tun_freebsd.rs  -- FreeBSD TUN via /dev/tun
├── routing/            -- routing table and DNS policy
│   ├── mod.rs          -- RoutingEngine; route_packet, route_dns_query
│   ├── table.rs        -- RouteTable; Cidr; longest-prefix match
│   ├── split.rs        -- SplitPolicy (Full / Split routing modes)
│   ├── dns.rs          -- DnsPolicy; DnsAction (UseDoh / UseSystemDns)
│   └── doh.rs          -- DohForwarder; DNS-over-HTTPS client
├── auth/               -- authentication backends
│   ├── mod.rs          -- AuthError; AuthInput; AuthIdentity; ProbeGatePolicy
│   ├── backend.rs      -- AuthCoordinator; AuthBackend trait dispatch
│   ├── common_crypto.rs -- shared Ed25519 verify helper (private module)
│   ├── static_key.rs   -- StaticKeyBackend (pre-shared key list)
│   ├── token.rs        -- TokenBackend; TokenClaims; HMAC-signed bearer tokens
│   └── certificate.rs  -- CertificateBackend; CertificateClaims; TrustAnchor
├── config/             -- configuration parsing and validation
│   ├── mod.rs
│   ├── parser.rs       -- key=value config file parser
│   ├── types.rs        -- AppConfig, ClientConfig, TransportConfig, StealthConfig,
│   │                      AuthConfig, CryptoConfig, RoutingConfig, DnsConfig
│   └── profiles/       -- built-in stealth profiles
│       ├── mod.rs      -- StealthProfile struct; load_profile(); ProfileError
│       ├── chrome_131.rs
│       ├── firefox_130.rs
│       └── safari_18.rs
├── telemetry/          -- logging and metrics
│   ├── mod.rs
│   ├── log.rs          -- EventCode; format_event; emit_health_probe
│   └── metrics.rs      -- MetricsRegistry; MetricsSnapshot
└── util/               -- shared primitive types and buffer utilities
    ├── mod.rs          -- ConnectionState, TransportMode, AuthMethod,
    │                      CamouflageMode, ConnectionSession, CryptoContext,
    │                      StealthProfile
    ├── buf.rs          -- ByteWriter, ByteCursor
    ├── ring_buf.rs     -- fixed-capacity ring buffer
    └── endian.rs       -- big-endian read/write helpers
```

---

## 3. Packet Processing Pipeline

The following describes the path of an inbound plaintext IP packet on the
client side (from the TUN interface to the peer server) and the reverse path
for a received encrypted frame.

### 3.1 Outbound Path (TUN -> Wire)

```
TUN adapter (tunnel::tun_*)
    |
    | Raw IP packet bytes
    v
tunnel::packet::parse_ip_header()
    |
    | Destination IP address
    v
routing::RoutingEngine::route_packet()
    |
    | RouteTarget::Tunnel  (RouteTarget::Bypass exits the pipeline)
    v
transport::frame::encode_frame()
    |
    | Frame { type: Data, sequence: N, payload: ip_bytes }
    | Header encoded big-endian: [type(1) | flags(1) | payload_len(2) | sequence(8)]
    v
crypto::aead::encrypt_chacha20poly1305()
    |
    | Ciphertext with 16-byte Poly1305 tag
    v
stealth::padding::apply_padding()        (if StealthProfile active)
stealth::timing::enforce_jitter()        (if StealthProfile active)
    |
    v
TransportStrategy::send()
    |
    | (UdpTls / TcpTls / QuicMask)
    v
Network socket
```

### 3.2 Inbound Path (Wire -> TUN)

```
Network socket
    |
    v
TransportStrategy::recv()
    |
    | Raw encrypted bytes
    v
transport::frame::decode_frame()
    |
    | DecodedFrame { frame, context }
    | Validates: minimum length, flags mask, payload_len <= 16384
    v
crypto::aead::decrypt_chacha20poly1305()
    |
    | Plaintext IP packet bytes
    v
routing::RoutingEngine::route_packet()   (sanity check on source)
    |
    v
TUN adapter write
```

### 3.3 FEC Path (UDP only)

When `FecController` is in `SingleParity` or `DoubleParity` mode, additional
parity shards are appended to outbound bursts. On the inbound path,
`recover_single_lost_shard` reconstructs a missing shard via XOR before
the frame is passed to the decrypt stage. FEC is automatically disabled
when the active transport kind is `TcpTls` (TCP provides reliable delivery).

---

## 4. State Machines

### 4.1 Connection Lifecycle (util::ConnectionState)

The `ConnectionState` enum governs the high-level lifecycle of a
`ConnectionSession`. The permitted transitions are:

```
Init
  |
  | (handshake started)
  v
Handshaking
  |
  | (Noise handshake completes -> HandshakeState::Established)
  v
Established <----+
  |              |
  | (rekey       | (rekey completes)
  |  triggered)  |
  v              |
Rekeying --------+
  |
  | (endpoint change detected)
  v
Migrating
  |
  | (new path verified)
  v
Established
  |
  | (CLOSE frame received or sent)
  v
Closing
  |
  v
Closed
```

Any state may transition to `Closed` on a fatal error.

### 4.2 Noise Handshake State Machine (noise::HandshakeState)

`NoiseSession::transition()` enforces the following valid edges. Any other
transition returns `SecurityError::InvalidHandshake`.

```
Init
  |
  | ClientHello received (ephemeral_public mixed into SymmetricState)
  v
EphemeralExchanged
  |
  | ServerHello received (ephemeral_public mixed into SymmetricState)
  | (no state enum change; seen_server_hello flag set in HandshakeMachine)
  |
  | AuthProof received (signature mixed into SymmetricState; ct_eq verified)
  v
Authenticated
  |
  | (automatic transition)
  v
Established
  |         ^
  | begin_  | finalize_
  | rekey() | rekey() increments key_epoch
  v         |
Rekeying ---+
```

Any state may transition to `Failed` on error.

Replay protection is enforced by boolean flags (`seen_client_hello`,
`seen_server_hello`) in `HandshakeMachine`. A second `ClientHello` or
`ServerHello` within the same session returns `SecurityError::ReplayDetected`.

### 4.3 Transport Mode Negotiation (transport::ModeNegotiator)

`ModeNegotiator` selects and falls back between transport kinds based on the
configured `TransportMode` and the outcome of each connection attempt
(`AttemptOutcome`).

```
TransportMode::Auto
    initial_kind() -> UdpTls

    UdpTls + TimedOut  -> next_kind() -> Some(TcpTls)
    UdpTls + Failed    -> next_kind() -> Some(TcpTls)
    TcpTls + any       -> next_kind() -> None (no further fallback)

TransportMode::Udp
    initial_kind() -> UdpTls
    any outcome    -> next_kind() -> None

TransportMode::Tcp
    initial_kind() -> TcpTls
    any outcome    -> next_kind() -> None

TransportMode::QuicMask
    initial_kind() -> QuicMask
    any outcome    -> next_kind() -> None
```

The fallback timeout (default 3 s, configurable via
`transport.fallback_timeout`) is held in `ModeNegotiator::fallback_timeout`.

---

## 5. Authentication Flow

Authentication occurs after the Noise handshake reaches `Established`. The
`AuthCoordinator` dispatches the inbound `AuthInput` (method + opaque payload
bytes) to the registered `AuthBackend` for that method. If authentication
fails, `evaluate_probe_gate` consults `ProbeGatePolicy`. When
`facade_on_auth_failure = true` (default), the connection is handed to
`stealth::facade` which serves a decoy HTTP or QUIC response, giving the
appearance of a legitimate web server to active probers.

```
AuthInput { method, payload }
    |
    v
AuthCoordinator::authenticate()
    |
    +-- StaticKeyBackend  (method = static_key)
    +-- TokenBackend      (method = token; HMAC-signed bearer token)
    +-- CertificateBackend (method = certificate; Ed25519-signed cert claims)
    |
    v
Result<AuthIdentity, AuthError>
    |
    v
evaluate_probe_gate(result, ProbeGatePolicy)
    |
    +-- AllowTunnel(AuthIdentity)  -> proceed to data path
    +-- ServeFacade                -> stealth::facade::serve_decoy()
    +-- Reject                     -> close connection
```

---

## 6. Cross-Platform Strategy

Apate targets Linux, macOS, Windows, and FreeBSD. Platform divergence is
confined to two modules:

**runtime::backend** — The event loop backend is selected at compile time:

| Platform | Backend     | Feature gated by         |
|----------|-------------|--------------------------|
| Linux    | epoll       | `cfg(target_os = "linux")` (default) |
| Linux    | io_uring    | `cfg(target_os = "linux")` (optional) |
| macOS    | kqueue      | `cfg(target_os = "macos")` |
| FreeBSD  | kqueue      | `cfg(target_os = "freebsd")` |
| Windows  | IOCP        | `cfg(windows)` |

**tunnel** — The TUN adapter is selected at compile time:

| Platform | Module         | Interface              |
|----------|----------------|------------------------|
| Linux    | `tun_linux`    | `/dev/net/tun` + ioctl |
| macOS    | `tun_macos`    | `/dev/tun*` + ioctl    |
| FreeBSD  | `tun_freebsd`  | `/dev/tun*`            |
| Windows  | `tun_windows`  | WinTUN via `windows-sys` |

All other modules (`crypto`, `noise`, `transport`, `auth`, `config`,
`routing`, `stealth`, `telemetry`, `util`) are platform-independent and
compile without conditional code.

The `windows-sys` dependency is target-gated in `Cargo.toml`:
```toml
[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61.2", features = [...] }
```

`libc` is declared as a universal dependency but its usage is isolated to the
POSIX tunnel adapters; it compiles on Windows but its POSIX symbols are unused.

---

## 7. Error Hierarchy

All error types derive `thiserror::Error` and implement `std::error::Error`.
The top-level `ApateError` in `lib.rs` aggregates module errors via `#[from]`:

```
ApateError
├── Config(ConfigError)
│   ├── MissingRequiredKey { key }
│   ├── InvalidValue { key }
│   └── UnsupportedKey { key }
├── Auth(AuthError)
│   ├── EmptyPayload
│   ├── UnsupportedBackend { method }
│   ├── Rejected
│   └── Internal
├── Frame(FrameError)
│   ├── Malformed
│   ├── UnsupportedType
│   ├── PayloadTooLarge
│   └── InvalidFlags
├── Transport(TransportError)
│   ├── NotConnected
│   ├── Timeout
│   └── Frame(FrameError)
├── Security(SecurityError)
│   ├── InvalidHandshake
│   ├── ReplayDetected
│   ├── KeyDerivationFailed
│   ├── CipherFailure
│   └── ConstantTimeVerificationFailed
└── Runtime(RuntimeError)
    ├── BackendUnavailable { backend }
    ├── EventLoopStartFailed
    └── ShutdownTimeout
```

Error messages never leak key material, token bytes, certificate private data,
or any secret-derived diagnostic. `AuthError::Rejected` carries a fixed string
("authentication rejected") regardless of why validation failed.
