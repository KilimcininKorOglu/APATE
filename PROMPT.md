# Apate — Claude Code Implementation Prompt

## Project Overview

Build **Apate**, a Rust stealth VPN protocol and runtime-focused tunnel system for hostile network environments.  
Core goals:

1. Minimal curated dependency set (strict supply-chain discipline)
2. Low-latency packet path (target protocol overhead <10μs in steady state)
3. DPI resistance (traffic camouflage, shaping, probe defense)

Scope for this build:

- Full v1.0 feature set
- Cross-platform target: Linux, macOS, Windows (FreeBSD compile path included)
- CLI + config control plane only (no external REST/gRPC management API)
- Authentication backends: static key, token, certificate

## Tech Stack

| Layer             | Technology                          | Version             |
|-------------------|-------------------------------------|---------------------|
| Language          | Rust                                | 1.93.x stable       |
| Build             | Cargo                               | 1.93.x              |
| Crypto AEAD       | chacha20poly1305, aes-gcm           | 0.10.1, 0.10.3      |
| KX + signatures   | x25519-dalek, ed25519-dalek, ml-kem | 2.0.1, 2.2.0, 0.2.3 |
| KDF/hash/security | blake3, subtle, zeroize             | 1.8.4, 2.6.1, 1.8.2 |
| RNG               | rand_core, rand_chacha              | 0.10.0, 0.10.0      |
| Sys/platform      | libc, windows-sys                   | 0.2.184, 0.61.2     |
| Testing           | cargo test, proptest, criterion     | criterion 0.8.2     |
| Security gates    | cargo-audit, cargo-deny             | latest stable       |
| CI/CD             | GitHub Actions                      | hosted              |

## Working Directory Rule

- Project root already exists and is current working directory.
- Do **not** create wrapper subfolder.
- Keep planning docs in root: `SPECIFICATION.md`, `IMPLEMENTATION.md`, `TASKS.md`, `BRANDING.md`, `PROMPT.md`.

## Project Structure

```text
.
├── Cargo.toml
├── Cargo.lock
├── build.rs
├── DEPS.md
├── deny.toml
├── .cargo/
│   └── config.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── cli/{mod.rs,args.rs,commands.rs}
│   ├── runtime/{mod.rs,reactor.rs,executor.rs,timer.rs,waker.rs}
│   ├── runtime/backend/{mod.rs,epoll.rs,io_uring.rs,kqueue.rs,iocp.rs}
│   ├── crypto/{mod.rs,aead.rs,kx.rs,sign.rs,kdf.rs,rng.rs}
│   ├── noise/{mod.rs,handshake.rs,state.rs,cipher_state.rs,symmetric_state.rs}
│   ├── transport/{mod.rs,frame.rs,ack.rs,loss.rs,congestion.rs,pacing.rs,mode.rs,connection.rs,fec.rs,migration.rs,udp_tls.rs,quic_mask.rs,tcp_tls.rs}
│   ├── stealth/{mod.rs,tls_camouflage.rs,quic_camouflage.rs,client_hello.rs,server_hello.rs,padding.rs,timing.rs,entropy.rs,facade.rs}
│   ├── tunnel/{mod.rs,packet.rs,tun_linux.rs,tun_macos.rs,tun_windows.rs,tun_freebsd.rs}
│   ├── routing/{mod.rs,table.rs,split.rs,dns.rs,doh.rs}
│   ├── auth/{mod.rs,backend.rs,static_key.rs,token.rs,certificate.rs}
│   ├── config/{mod.rs,parser.rs,types.rs}
│   ├── config/profiles/{mod.rs,chrome_131.rs,firefox_130.rs,safari_18.rs}
│   ├── telemetry/{mod.rs,log.rs,metrics.rs}
│   └── util/{mod.rs,buf.rs,ring_buf.rs,endian.rs}
├── tests/
│   ├── integration/{handshake.rs,tunnel_data.rs,fallback.rs,migration.rs}
│   ├── property/{frame_roundtrip.rs,parser_fuzz_harness.rs}
│   └── fixtures/{keys,certs,profiles}
├── fuzz/
│   ├── Cargo.toml
│   └── fuzz_targets/{frame_parser.rs,config_parser.rs,handshake.rs}
├── benches/{packet_path.rs,crypto_wrappers.rs,handshake.rs}
├── docs/{architecture.md,wire-format.md,operations.md}
└── .github/workflows/{ci.yml,security.yml,release.yml}
```

## Dependencies and Base Configuration

Create `Cargo.toml` with exact versions:

```toml
[package]
name = "apate"
version = "0.1.0"
edition = "2024"

[dependencies]
chacha20poly1305 = { version = "0.10.1", default-features = false, features = ["alloc"] }
aes-gcm          = { version = "0.10.3", default-features = false, features = ["alloc"] }
x25519-dalek     = { version = "2.0.1", default-features = false, features = ["static_secrets"] }
ed25519-dalek    = { version = "2.2.0", default-features = false, features = ["fast"] }
ml-kem           = { version = "0.2.3", default-features = false, features = ["ml-kem-768"] }
blake3           = { version = "1.8.4", default-features = false }
rand_core        = { version = "0.10.0", default-features = false, features = ["os_rng"] }
rand_chacha      = { version = "0.10.0", default-features = false }
zeroize          = { version = "1.8.2", default-features = false, features = ["derive"] }
subtle           = { version = "2.6.1", default-features = false }
libc             = { version = "0.2.184", default-features = false }
hashbrown        = "0.16.0"
thiserror        = "2.0.11"
bytes            = "1.10.1"
tracing          = "0.1.41"
tracing-subscriber = "0.3.20"

[target.'cfg(windows)'.dependencies]
windows-sys      = { version = "0.61.2", features = ["Win32_Networking_WinSock", "Win32_System_IO", "Win32_Foundation"] }

[dev-dependencies]
proptest = "1.9.0"
criterion = "0.8.2"

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

Create `.cargo/config.toml`:

```toml
[build]
target-dir = "target"

[term]
verbose = true
```

Create `deny.toml`:

```toml
[graph]
all-features = true

[advisories]
version = 2
yanked = "deny"
ignore = []

[licenses]
version = 2
allow = ["MIT", "Apache-2.0", "BSD-3-Clause", "CC0-1.0", "Unicode-3.0"]
deny = ["GPL-2.0", "GPL-3.0", "AGPL-3.0"]

[bans]
multiple-versions = "deny"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

Create `build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
```

## Core Design Patterns (Use Exactly)

### 1) Ports and Adapters for auth and platform layers

```rust
pub trait AuthBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError>;
}

pub trait TunAdapter {
    fn read_packet(&mut self, buf: &mut [u8]) -> Result<usize, TunError>;
    fn write_packet(&mut self, buf: &[u8]) -> Result<(), TunError>;
}
```

### 2) State machine for connection lifecycle

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Init,
    Handshaking,
    Established,
    Rekeying,
    Migrating,
    Closing,
    Closed,
}
```

### 3) Strategy for transport modes

```rust
pub trait TransportStrategy {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError>;
    fn recv(&mut self) -> Result<Option<Frame>, TransportError>;
}
```

### 4) Packet processing pipeline

```rust
pub trait PacketStage {
    fn run(&self, ctx: PacketCtx) -> Result<PacketCtx, StageError>;
}
```

## Functional and Security Requirements to Implement

- Handshake: authenticated key establishment + transition to transport mode
- Transport: UDP-first `auto` mode with TCP fallback; explicit forced mode support
- Reliability: ACK/loss recovery/pacing/congestion with deterministic behavior
- Stealth: TLS camouflage, QUIC-mask mode, timing and length shaping
- Probe defense: route unauthenticated probes to web-facade path
- Routing: full and split tunnel modes, DNS policy and DoH mode
- Auth backends: static key, token, certificate
- Session resilience: rekey and migration support
- Security hygiene: zeroize secrets, constant-time checks, sanitized error outputs

## Data Model (Runtime State)

Implement these core structs in typed modules:

```rust
pub struct ConnectionSession {
    pub connection_id: [u8; 16],
    pub state: ConnectionState,
    pub transport_mode: TransportMode,
    pub auth_method: AuthMethod,
    pub established_at_unix: u64,
    pub peer_endpoint: String,
}

pub struct CryptoContext {
    pub key_epoch: u64,
    pub tx_nonce_counter: u64,
    pub rekey_interval_secs: u32,
    pub rekey_interval_bytes: u64,
}

pub struct StealthProfile {
    pub name: String,
    pub mode: CamouflageMode,
    pub packet_min: u16,
    pub packet_max: u16,
    pub jitter_ms_max: u16,
}
```

## Protocol Message Surface

Implement frame/message handling for:

- HANDSHAKE
- DATA
- ACK
- REKEY
- MIGRATE
- CLOSE

Malformed frames must be dropped with bounded-cost error path and no panic.

## Implementation Order

Follow this exact order. Do not skip ahead.

### Step 1: Scaffolding

**Files:** root config files + module directories + `src/lib.rs` + `src/main.rs`.

**Tests:** `cargo +stable check`, `cargo +stable test`.

### Step 2: Core types and errors

**Files:** `src/*/mod.rs`, `src/config/types.rs`, error enums in all major modules.

**Tests:** compilation + unit tests for state transitions.

### Step 3: Config parser and profile loader

**Files:** `src/config/{mod.rs,parser.rs,types.rs}`, `src/config/profiles/mod.rs`.

**Edge cases:** unknown keys, invalid enum values, malformed numeric bounds.

### Step 4: Runtime core

**Files:** `src/runtime/*`, backend trait + host backend stubs.

**Checkpoint:** runtime event loop starts/stops cleanly.

### Step 5: Crypto wrappers

**Files:** `src/crypto/*`.

**Tests:** AEAD and KDF vectors; key erase behavior.

### Step 6: Handshake state machine

**Files:** `src/noise/*`.

**Tests:** success/failure/replay attempts.

### Step 7: Frame codec

**Files:** `src/transport/frame.rs`, `src/util/buf.rs`, property tests.

**Tests:** roundtrip and malformed frame rejection.

### Step 8: Transport engine and mode negotiation

**Files:** `src/transport/{connection.rs,mode.rs,udp_tls.rs,tcp_tls.rs,quic_mask.rs}`.

**Behavior:** UDP-first with timeout fallback to TCP.

### Step 9: ACK/loss/pacing/congestion

**Files:** `src/transport/{ack.rs,loss.rs,congestion.rs,pacing.rs}`.

### Step 10: Tunnel adapters Linux/macOS

**Files:** `src/tunnel/{mod.rs,packet.rs,tun_linux.rs,tun_macos.rs}`.

### Step 11: Tunnel adapters Windows/FreeBSD + backend glue

**Files:** `src/tunnel/{tun_windows.rs,tun_freebsd.rs}`, `src/runtime/backend/{iocp.rs,kqueue.rs}`.

### Step 12: Routing and DNS policy

**Files:** `src/routing/{mod.rs,table.rs,split.rs,dns.rs,doh.rs}`.

### Step 13: Profile registry

**Files:** `src/config/profiles/{chrome_131.rs,firefox_130.rs,safari_18.rs,mod.rs}`, `src/stealth/mod.rs`.

### Step 14: TLS camouflage and shaping

**Files:** `src/stealth/{tls_camouflage.rs,client_hello.rs,server_hello.rs,padding.rs,timing.rs}`.

### Step 15: QUIC-mask path

**Files:** `src/stealth/quic_camouflage.rs`, `src/transport/quic_mask.rs`.

### Step 16: Active probing defense

**Files:** `src/stealth/facade.rs`, glue in `src/auth/mod.rs`.

### Step 17: Static key backend

**Files:** `src/auth/{backend.rs,static_key.rs,mod.rs}`.

### Step 18: Token backend

**Files:** `src/auth/token.rs`.

### Step 19: Certificate backend

**Files:** `src/auth/certificate.rs`.

### Step 20: Rekey and migration

**Files:** `src/noise/state.rs`, `src/transport/migration.rs`, `src/transport/connection.rs`.

### Step 21: FEC and adaptive recovery

**Files:** `src/transport/fec.rs` + connection integration.

### Step 22: Full integration suite

**Files:** `tests/integration/{handshake.rs,tunnel_data.rs,fallback.rs,migration.rs}`.

### Step 23: Fuzz targets

**Files:** `fuzz/Cargo.toml`, `fuzz/fuzz_targets/*`.

### Step 24: Benchmarks

**Files:** `benches/{packet_path.rs,crypto_wrappers.rs,handshake.rs}`.

### Step 25: Security policy gates

**Files:** `deny.toml`, `.github/workflows/security.yml`, `DEPS.md`.

### Step 26: Cross-platform CI and release workflow

**Files:** `.github/workflows/{ci.yml,release.yml,security.yml}`.

### Step 27: Packaging and operational metrics

**Files:** `src/telemetry/{log.rs,metrics.rs}`, optional `Dockerfile`, `docs/operations.md`.

### Step 28: Final release candidate validation

Run full verification and fix blockers only.

## Error Handling Rules

Map errors into deterministic categories:

- `ConfigError`
- `AuthError`
- `FrameError`
- `TransportError`
- `SecurityError`
- `RuntimeError`

Never leak keys, token material, certificate private data, or internal secret-dependent diagnostics.

## Authentication Flow

```text
1. Parse and validate handshake + auth payload
2. Route payload to AuthCoordinator
3. Evaluate enabled backends (static/token/certificate)
4. On success: derive transport keys and transition to Established
5. On failure: reject connection and optionally route to facade behavior
```

## Configuration Keys to Support

| Key                           | Required | Default      | Description                    |
|-------------------------------|----------|--------------|--------------------------------|
| `client.server`               | client   | none         | Server endpoint                |
| `transport.mode`              | no       | `auto`       | `auto`/`udp`/`tcp`             |
| `transport.fallback_timeout`  | no       | `3`          | UDP fallback timeout seconds   |
| `stealth.profile`             | no       | `chrome_131` | Built-in profile name          |
| `stealth.profile_path`        | no       | empty        | Optional profile override path |
| `auth.methods`                | server   | none         | Enabled auth backends          |
| `crypto.post_quantum`         | no       | `true`       | Hybrid KX toggle               |
| `crypto.rekey_interval_secs`  | no       | `60`         | Time-based rekey trigger       |
| `crypto.rekey_interval_bytes` | no       | `1073741824` | Byte-based rekey trigger       |
| `routing.mode`                | no       | `full`       | `full`/`split`                 |
| `dns.mode`                    | no       | `doh`        | `doh`/`plain`/fallback         |

## Testing Requirements

Write and maintain:

- Unit tests for crypto wrappers, parser, state transitions
- Property tests for frame codec
- Integration tests for handshake, data path, fallback, migration, auth modes
- Fuzz targets for frame/parser/handshake input boundaries
- Benchmarks for packet path and handshake compute

## CI Workflows

### ci.yml (required stages)

1. fmt check
2. clippy with deny warnings
3. unit + integration tests
4. cross-target build matrix

### security.yml (required stages)

1. `cargo audit`
2. `cargo deny check`

### release.yml (required stages)

1. build release artifacts for target matrix
2. checksum generation
3. artifact upload

## Final Quality Gates

Before completion, all must pass:

- [ ] `cargo +stable fmt --all --check`
- [ ] `cargo +stable clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo +stable test`
- [ ] `cargo +stable check --all-targets`
- [ ] `cargo audit`
- [ ] `cargo deny check`
- [ ] Release build succeeds for Linux/macOS/Windows targets

If any gate fails, fix root cause and rerun all gates.
