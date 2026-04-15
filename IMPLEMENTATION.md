# Apate — Implementation Plan

> Technical blueprint derived from `SPECIFICATION.md`.

## 1. Tech Stack

### 1.1 Stack Summary

| Layer                  | Technology                                | Version                               | Rationale                                                                            |
|------------------------|-------------------------------------------|---------------------------------------|--------------------------------------------------------------------------------------|
| Language               | Rust                                      | stable `1.93.x` baseline              | Meets low-level systems, safety, and performance requirements from SPEC §1.1 and §10 |
| Toolchain              | Cargo + rustup                            | Cargo `1.93.x`                        | Standard Rust build ecosystem with reproducible lockfiles                            |
| Runtime model          | Custom async reactor/executor             | Project code                          | Required by SPEC §11.1 dependency constraint and latency goals in SPEC §10.1         |
| Crypto AEAD            | `chacha20poly1305`, `aes-gcm`             | `0.10.1`, `0.10.3`                    | Stable audited crates aligned with curated-dependency philosophy                     |
| Key exchange/signature | `x25519-dalek`, `ed25519-dalek`, `ml-kem` | `2.0.1`, `2.2.0`, `0.2.3`             | Required by hybrid auth/key model in SPEC §3.1 and §8.1                              |
| KDF/hash               | `blake3`, `subtle`, `zeroize`             | `1.8.4`, `2.6.1`, `1.8.2`             | Fast key derivation + constant-time + key erasure semantics                          |
| Platform abstraction   | `libc`, `windows-sys`                     | `0.2.184`, `0.61.2`                   | Direct syscall/Winsock access with minimal abstraction                               |
| RNG                    | `rand_core`, `rand_chacha`                | `0.10.0`, `0.10.0`                    | Deterministic and secure random generation split                                     |
| Testing/bench          | `cargo test`, `proptest`, `criterion`     | latest stable (`0.8.2` for criterion) | Needed for correctness and performance verification in SPEC §10                      |
| Linting/format         | `clippy`, `rustfmt`                       | Rust toolchain bundled                | Enforces consistency and catches correctness issues                                  |
| CI/CD                  | GitHub Actions                            | hosted                                | Matches repository hosting workflow and easy multi-target matrix                     |
| Container              | Docker (optional deploy artifact)         | current stable                        | Optional packaging for server deployments from SPEC §9.2                             |

### 1.2 Key Technical Decisions

#### Decision: Modular Monolith Codebase
- **Context**: SPEC §4 requires many tightly-coupled subsystems with shared low-level concerns.
- **Options Considered**:
  1. **Single modular crate workspace**: Simple deploy, strong boundaries via modules / Cons: discipline required.
  2. **Microservices split**: Independent scaling / Cons: high distributed complexity for early phase.
  3. **Plugin-based runtime core**: Flexible extension / Cons: larger attack surface and complexity.
- **Choice**: Single modular monolith (workspace with clear crate/module boundaries).
- **Rationale**: Best fit for full v1.0 scope while keeping latency-sensitive internals cohesive.
- **Consequences**: Need strict module dependency rules and architectural linting.

#### Decision: Ports-and-Adapters for Auth and Platform I/O
- **Context**: SPEC §3.5 and §9.1 require multiple auth backends and cross-platform tunnel/network support.
- **Options Considered**:
  1. **Ports-and-adapters**: Testable and swappable / Cons: more interfaces.
  2. **Direct concrete wiring**: Fast initial coding / Cons: hard to test and swap.
  3. **Macro-heavy abstraction**: Less boilerplate / Cons: readability/debug cost.
- **Choice**: Ports-and-adapters.
- **Rationale**: Required for static/token/certificate backends and OS-specific tunnel/network adapters.
- **Consequences**: Slight boilerplate increase, major long-term maintainability gain.

#### Decision: Explicit State Machines for Session/Connection
- **Context**: SPEC §3.1, §3.2, and §3.3 need strict lifecycle guarantees.
- **Options Considered**:
  1. **Typed state machine enums**: Safe transitions / Cons: more code paths to model.
  2. **Boolean flags with guards**: Quick start / Cons: invalid state risk.
  3. **Table-driven transitions**: Flexible / Cons: less compile-time safety.
- **Choice**: Typed state machine enums + transition functions.
- **Rationale**: Prevents illegal transitions in handshake, migration, rekey, and close flows.
- **Consequences**: Upfront modeling effort, fewer production logic bugs.

#### Decision: Strategy Pattern for Transport and Stealth Profiles
- **Context**: SPEC §3.2 and §3.3 require runtime mode switching and profile selection.
- **Options Considered**:
  1. **Strategy interfaces**: Runtime swappable behavior / Cons: virtual dispatch overhead.
  2. **Monolithic match blocks**: Fewer abstractions / Cons: hard to evolve safely.
  3. **Compile-time feature matrix only**: Lean binaries / Cons: weak runtime flexibility.
- **Choice**: Strategy traits with enum-dispatch where needed.
- **Rationale**: Enables deterministic mode selection with manageable complexity.
- **Consequences**: Must benchmark dispatch overhead in hot paths.

#### Decision: No External Management API in v1.0
- **Context**: User-selected scope and SPEC §3.6/§6.
- **Options Considered**:
  1. **CLI + config only**: Minimal attack surface / Cons: less remote automation.
  2. **REST control API**: automation-friendly / Cons: security + maintenance overhead.
  3. **gRPC control API**: typed contracts / Cons: dependency and ops complexity.
- **Choice**: CLI + config only.
- **Rationale**: Keeps focus on protocol reliability and stealth objectives.
- **Consequences**: Automation flows rely on local process orchestration.

#### Decision: Curated Dependency Governance
- **Context**: SPEC §1.3 and §11.1 enforce minimal dependencies and auditability.
- **Options Considered**:
  1. **Strict dependency budget + deny/audit gates**: high assurance / Cons: slower dependency upgrades.
  2. **Ecosystem-standard broad usage**: faster development / Cons: supply-chain surface growth.
  3. **Std-only extreme**: minimal dependencies / Cons: cryptographic and platform risk.
- **Choice**: Strict curated dependency model.
- **Rationale**: Aligns with project identity and threat model.
- **Consequences**: Requires explicit DEPS review workflow.

### 1.3 Dependency Inventory

| Package          | Purpose                                  | License               | Justification                           |
|------------------|------------------------------------------|-----------------------|-----------------------------------------|
| chacha20poly1305 | ChaCha20-Poly1305 AEAD                   | MIT OR Apache-2.0     | Audited AEAD for non-AES hardware paths |
| aes-gcm          | AES-256-GCM AEAD                         | MIT OR Apache-2.0     | Hardware-accelerated AEAD path          |
| x25519-dalek     | X25519 key agreement                     | BSD-3-Clause          | Mature constant-time ECDH               |
| ed25519-dalek    | Ed25519 signatures                       | BSD-3-Clause          | Strong signature ecosystem fit          |
| ml-kem           | Post-quantum KEM (ML-KEM-768)            | MIT OR Apache-2.0     | Hybrid PQ requirement in SPEC §8        |
| blake3           | Fast hash/KDF primitive base             | CC0-1.0 OR Apache-2.0 | High-throughput key derivation base     |
| rand_core        | RNG traits and OS entropy adapters       | MIT OR Apache-2.0     | Minimal secure randomness interface     |
| rand_chacha      | ChaCha-based deterministic DRBG          | MIT OR Apache-2.0     | Controlled randomness streams           |
| zeroize          | Sensitive memory zeroization             | MIT OR Apache-2.0     | Key material lifecycle hardening        |
| subtle           | Constant-time utilities                  | MIT OR Apache-2.0     | Side-channel resistant comparisons      |
| libc             | Portable syscall bindings                | MIT OR Apache-2.0     | Required for thin platform integration  |
| windows-sys      | Windows system APIs (conditional target) | MIT OR Apache-2.0     | IOCP and networking support for Windows |

Dependency policy: direct deps ≤ 15, transitive deps ≤ 50, no tokio/hyper/runtime frameworks, mandatory `cargo audit` + `cargo deny`.

## 2. Design Patterns

### 2.1 Architectural Pattern: Modular Monolith

**Why:** Maps to SPEC §4.1 multi-subsystem architecture while keeping single deployable footprint and tight latency control.

**Application:** One repository with bounded internal modules (`runtime`, `transport`, `stealth`, `auth`, `tunnel`, `routing`) and explicit dependency directions.

**Code Sketch:**
```rust
pub struct ApateNode {
    runtime: runtime::Reactor,
    transport: transport::Engine,
    stealth: stealth::CamouflageEngine,
    auth: auth::AuthCoordinator,
}

impl ApateNode {
    pub fn run(self) -> Result<(), ApateError> {
        self.runtime.drive(self.transport, self.stealth, self.auth)
    }
}
```

### 2.2 Pattern: Ports & Adapters (Hexagonal)

**Why:** Needed for pluggable auth backends and OS-specific tunnel adapters in SPEC §3.5 and §9.1.

**Code Sketch:**
```rust
pub trait AuthBackend {
    fn authenticate(&self, input: AuthInput) -> Result<AuthIdentity, AuthError>;
}

pub struct AuthCoordinator {
    backends: Vec<Box<dyn AuthBackend + Send + Sync>>,
}

impl AuthCoordinator {
    pub fn verify(&self, input: AuthInput) -> Result<AuthIdentity, AuthError> {
        self.backends.iter().find_map(|b| b.authenticate(input.clone()).ok()).ok_or(AuthError::Denied)
    }
}
```

### 2.3 Pattern: Strategy

**Why:** Runtime-selectable transport and stealth mode behavior from SPEC §3.2 and §3.3.

**Code Sketch:**
```rust
pub trait TransportStrategy {
    fn send(&mut self, frame: Frame) -> Result<(), TransportError>;
    fn poll_recv(&mut self) -> Result<Option<Frame>, TransportError>;
}

pub enum TransportMode {
    UdpTls(UdpTlsMode),
    QuicMask(QuicMaskMode),
    TcpTls(TcpTlsMode),
}
```

### 2.4 Pattern: State Machine

**Why:** Session correctness for handshake, rekey, migration, and teardown in SPEC §3.1.

**Code Sketch:**
```rust
pub enum ConnectionState {
    Init,
    Handshaking,
    Established,
    Rekeying,
    Migrating,
    Closing,
    Closed,
}

pub fn transition(state: ConnectionState, event: Event) -> Result<ConnectionState, StateError> { /* ... */ }
```

### 2.5 Pattern: Middleware/Pipeline

**Why:** Ordered packet path from SPEC §3.2–§3.4 must stay composable and testable.

**Code Sketch:**
```rust
pub trait PacketStage {
    fn run(&self, packet: PacketCtx) -> Result<PacketCtx, StageError>;
}

pub fn process(stages: &[Box<dyn PacketStage>], mut ctx: PacketCtx) -> Result<PacketCtx, StageError> {
    for stage in stages { ctx = stage.run(ctx)?; }
    Ok(ctx)
}
```

## 3. Project Structure

### 3.1 Directory Layout

> Root is current working directory.

```text
.
├── SPECIFICATION.md
├── IMPLEMENTATION.md
├── TASKS.md
├── BRANDING.md
├── PROMPT.md
├── Cargo.toml
├── Cargo.lock
├── DEPS.md
├── build.rs
├── .cargo/
│   └── config.toml
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs
│   │   └── commands.rs
│   ├── runtime/
│   │   ├── mod.rs
│   │   ├── reactor.rs
│   │   ├── executor.rs
│   │   ├── timer.rs
│   │   ├── waker.rs
│   │   └── backend/
│   │       ├── mod.rs
│   │       ├── epoll.rs
│   │       ├── io_uring.rs
│   │       ├── kqueue.rs
│   │       └── iocp.rs
│   ├── crypto/
│   │   ├── mod.rs
│   │   ├── aead.rs
│   │   ├── kx.rs
│   │   ├── sign.rs
│   │   ├── kdf.rs
│   │   └── rng.rs
│   ├── noise/
│   │   ├── mod.rs
│   │   ├── handshake.rs
│   │   ├── state.rs
│   │   ├── cipher_state.rs
│   │   └── symmetric_state.rs
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── mode.rs
│   │   ├── connection.rs
│   │   ├── frame.rs
│   │   ├── ack.rs
│   │   ├── congestion.rs
│   │   ├── pacing.rs
│   │   ├── fec.rs
│   │   ├── migration.rs
│   │   ├── udp_tls.rs
│   │   ├── quic_mask.rs
│   │   └── tcp_tls.rs
│   ├── stealth/
│   │   ├── mod.rs
│   │   ├── tls_camouflage.rs
│   │   ├── quic_camouflage.rs
│   │   ├── client_hello.rs
│   │   ├── server_hello.rs
│   │   ├── padding.rs
│   │   ├── timing.rs
│   │   ├── entropy.rs
│   │   └── facade.rs
│   ├── tunnel/
│   │   ├── mod.rs
│   │   ├── packet.rs
│   │   ├── tun_linux.rs
│   │   ├── tun_macos.rs
│   │   ├── tun_windows.rs
│   │   └── tun_freebsd.rs
│   ├── routing/
│   │   ├── mod.rs
│   │   ├── table.rs
│   │   ├── split.rs
│   │   ├── dns.rs
│   │   └── doh.rs
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── backend.rs
│   │   ├── static_key.rs
│   │   ├── token.rs
│   │   └── certificate.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   ├── types.rs
│   │   └── profiles/
│   │       ├── mod.rs
│   │       ├── chrome_131.rs
│   │       ├── firefox_130.rs
│   │       └── safari_18.rs
│   ├── telemetry/
│   │   ├── mod.rs
│   │   ├── log.rs
│   │   └── metrics.rs
│   └── util/
│       ├── mod.rs
│       ├── buf.rs
│       ├── ring_buf.rs
│       └── endian.rs
├── tests/
│   ├── integration/
│   │   ├── handshake.rs
│   │   ├── tunnel_data.rs
│   │   ├── migration.rs
│   │   └── fallback.rs
│   ├── fixtures/
│   │   ├── certs/
│   │   ├── keys/
│   │   └── profiles/
│   └── property/
│       ├── frame_roundtrip.rs
│       └── parser_fuzz_harness.rs
├── fuzz/
│   ├── Cargo.toml
│   └── fuzz_targets/
│       ├── frame_parser.rs
│       ├── config_parser.rs
│       └── handshake.rs
├── docs/
│   ├── architecture.md
│   ├── wire-format.md
│   └── operations.md
└── .github/
    └── workflows/
        ├── ci.yml
        ├── security.yml
        └── release.yml
```

**Structural Philosophy:**
- Layer + domain hybrid: keep low-level systems modules isolated, expose narrow interfaces.
- Platform-specific code stays inside adapter modules.
- Property/fuzz/integration tests separated by test type for targeted CI stages.

### 3.2 Module Breakdown

#### Module: Runtime
- **Path**: `src/runtime/`
- **Responsibility**: Event loop, scheduling, and timer orchestration.
- **Exports**: `Reactor`, `Executor`, `RuntimeHandle`.
- **Imports**: `util`, platform backend modules.
- **Key Files**:
  - `reactor.rs` — poller abstraction and event dispatch
  - `backend/*.rs` — OS-specific readiness/completion engines

#### Module: Crypto + Noise
- **Path**: `src/crypto/`, `src/noise/`
- **Responsibility**: Key exchange, key derivation, symmetric encryption, signature checks.
- **Exports**: `Cipher`, `HandshakeEngine`, `TransportKeys`.
- **Imports**: RustCrypto crates, `util`.
- **Key Files**:
  - `crypto/kx.rs` — hybrid KX composition
  - `noise/handshake.rs` — authenticated handshake lifecycle

#### Module: Transport
- **Path**: `src/transport/`
- **Responsibility**: Frame protocol, mode selection, reliability, pacing, migration.
- **Exports**: `TransportEngine`, `FrameCodec`, `ModeNegotiator`.
- **Imports**: `crypto`, `runtime`, `stealth`.
- **Key Files**:
  - `frame.rs` — binary frame encode/decode
  - `mode.rs` — UDP-first with TCP fallback policy

#### Module: Stealth
- **Path**: `src/stealth/`
- **Responsibility**: Wire camouflage, timing/size shaping, probe facade behavior.
- **Exports**: `CamouflageEngine`, `StealthProfileRuntime`.
- **Imports**: `transport`, `config`.
- **Key Files**:
  - `tls_camouflage.rs` — TLS record wrapping
  - `facade.rs` — non-auth probe response path

#### Module: Tunnel + Routing
- **Path**: `src/tunnel/`, `src/routing/`
- **Responsibility**: TUN I/O, packet parse, route policy, DNS handling.
- **Exports**: `TunDevice`, `Router`, `DnsPolicy`.
- **Imports**: platform adapters, `transport`.
- **Key Files**:
  - `tun_linux.rs`/`tun_macos.rs`/`tun_windows.rs` — OS adapters
  - `table.rs` — route lookup structure

#### Module: Auth
- **Path**: `src/auth/`
- **Responsibility**: Backend-agnostic authentication orchestration.
- **Exports**: `AuthBackend`, `AuthCoordinator`, backend implementations.
- **Imports**: `crypto`, `config`.
- **Key Files**:
  - `backend.rs` — trait contracts
  - `token.rs` / `certificate.rs` / `static_key.rs` — backend adapters

#### Module: Config
- **Path**: `src/config/`
- **Responsibility**: Config parse, validation, profile loading, reload hooks.
- **Exports**: `AppConfig`, `ConfigLoader`.
- **Imports**: `auth`, `transport`, `stealth`.
- **Key Files**:
  - `parser.rs` — strict TOML-subset parser
  - `profiles/mod.rs` — built-in + file override selection

### 3.3 Module Dependency Graph

```text
cli/main
   ↓
config ────────────────┐
   ↓                   │
auth   runtime         │
  ↓      ↓             │
noise → transport → stealth
           ↓
tunnel → routing
           ↓
       telemetry
```

Rules:
- `crypto/noise` has no dependency on `cli` or `telemetry`.
- `transport` never depends directly on platform-specific tunnel adapters.
- `stealth` does not own auth decisions; it only consumes validated context.

## 4. Data Layer

### 4.1 State Schema (In-Memory + File-Backed)

No primary relational database in v1.0. Data model is runtime-state dominant plus configuration/auth material files.

```rust
pub struct SessionStore {
    pub sessions: hashbrown::HashMap<ConnectionId, ConnectionSession>,
}

pub struct AuthMaterialStore {
    pub static_keys_path: PathBuf,
    pub token_secret_path: PathBuf,
    pub certificate_ca_path: PathBuf,
}
```

### 4.2 Persistence & Reload Strategy

- Configuration and profile files loaded at startup.
- Profile/runtime config hot-reload supported through signal-trigger path where platform supports it.
- Session state ephemeral; restart clears active sessions.
- Auth material rotation supported by reloading files and revalidating future connections.

### 4.3 Data Access Pattern

Repository-style traits over storage sources (memory/file) for testability.

```rust
pub trait SessionRepository {
    fn insert(&mut self, session: ConnectionSession);
    fn get(&self, id: &ConnectionId) -> Option<&ConnectionSession>;
    fn remove(&mut self, id: &ConnectionId) -> Option<ConnectionSession>;
}
```

### 4.4 Caching Strategy

- Hot-path caches: route lookup cache, profile parameter cache, reusable crypto context buffers.
- Cache invalidation: config/profile epoch bump invalidates dependent cached structures.

## 5. API Implementation

### 5.1 Control Surface Structure (CLI)

| Command / Signal            | Module Handler             | Description                          |
|-----------------------------|----------------------------|--------------------------------------|
| `apate-client --config ...` | `cli::commands::runClient` | Boot client runtime and tunnel loop  |
| `apate-server --config ...` | `cli::commands::runServer` | Boot server listeners and auth stack |
| Runtime reload signal       | `config::ConfigLoader`     | Reload profile/config subset safely  |

### 5.2 Protocol Message Contract

| Message Type | Handler Module          | Purpose                           |
|--------------|-------------------------|-----------------------------------|
| HANDSHAKE    | `noise::handshake`      | Session key establishment         |
| DATA         | `transport::connection` | Tunnel payload delivery           |
| ACK          | `transport::ack`        | Loss tracking and recovery        |
| REKEY        | `noise::state`          | Key rotation synchronization      |
| MIGRATE      | `transport::migration`  | Endpoint update without reconnect |
| CLOSE        | `transport::connection` | Graceful termination              |

### 5.3 Validation Approach

- Config inputs validated during parse and semantic validation pass.
- Frame parser performs strict bounds and type checks before dispatch.
- Auth handlers validate backend-specific structures before acceptance.

### 5.4 Authentication Flow

```text
1. Client sends handshake material + auth payload
2. Server routes payload to AuthCoordinator
3. Enabled backend validates credentials
4. On success, handshake finalizes transport keys
5. Connection enters Established state
```

## 6. Error Handling Strategy

### 6.1 Error Classification

| Category       | Example                         | Action                              |
|----------------|---------------------------------|-------------------------------------|
| ConfigError    | Invalid profile value           | Fail startup/reload with clear code |
| AuthError      | Invalid token/certificate       | Reject connection, audit log        |
| FrameError     | Malformed frame length/type     | Drop packet, increment abuse metric |
| TransportError | Send/receive path failure       | Retry/fallback/close by policy      |
| SecurityError  | Nonce reuse or invariant breach | Immediate session teardown          |
| RuntimeError   | Reactor backend failure         | Controlled shutdown                 |

### 6.2 Error Propagation

- Domain error enums in each module.
- Boundary mappers convert internal errors into CLI/log-safe codes.
- Sensitive fields redacted before logging.

## 7. Configuration

### 7.1 Config Sources

Priority: built-in defaults → config file → CLI overrides.

### 7.2 Config Schema

| Key                           | Type    | Default      | Description                        |
|-------------------------------|---------|--------------|------------------------------------|
| `client.server`               | string  | none         | Server endpoint for client mode    |
| `transport.mode`              | enum    | `auto`       | Mode strategy (`auto`,`udp`,`tcp`) |
| `stealth.profile`             | string  | `chrome_131` | Built-in profile selection         |
| `stealth.profile_path`        | string  | empty        | Optional custom profile path       |
| `auth.methods`                | array   | required     | Enabled backend list on server     |
| `crypto.post_quantum`         | bool    | `true`       | Hybrid KX enablement               |
| `crypto.rekey_interval_secs`  | integer | `60`         | Time-based rekey trigger           |
| `crypto.rekey_interval_bytes` | integer | `1073741824` | Byte-based rekey trigger           |
| `routing.mode`                | enum    | `full`       | Full or split tunnel               |
| `dns.mode`                    | enum    | `doh`        | DNS policy mode                    |

## 8. Testing Strategy

### 8.1 Test Pyramid

| Level       | Tooling                  | Scope                                                 |
|-------------|--------------------------|-------------------------------------------------------|
| Unit        | `cargo test`, `proptest` | Frame codec, state transitions, parser validation     |
| Integration | `cargo test --test ...`  | Client/server handshake, fallback, routing, auth flow |
| Fuzz        | `cargo fuzz`             | Frame parser, config parser, handshake parser         |
| Benchmark   | `criterion`              | Packet path overhead, crypto wrapper throughput       |

### 8.2 Test Patterns

- AAA structure for all tests.
- Test fixtures for keys/certs/profiles under `tests/fixtures`.
- Deterministic RNG seeds for replayable failure scenarios.

### 8.3 CI Pipeline

```text
PR/Push
  → fmt check
  → clippy (deny warnings)
  → unit tests
  → integration tests
  → fuzz smoke target
  → security gates (cargo audit + cargo deny)
  → release build (linux/macos/windows matrix)
```

## 9. Security Implementation

### 9.1 Input Sanitization Points

- Config parser boundaries
- Handshake/auth payload decoding
- Frame decode and control message decode
- DNS and routing policy input parsing

### 9.2 Secret Management

- Local development: file paths outside repository, never committed.
- Production: injected secrets via host secret store or mounted files.
- Runtime never logs raw key/token/cert private values.

### 9.3 Security Controls

- Constant-time comparisons in auth/crypto boundaries.
- Key material zeroization on drop/rekey.
- Connection abuse counters with policy-based rejection thresholds.
- Probe facade separation from authenticated tunnel path.

## 10. Deployment

### 10.1 Build Commands

```bash
cargo +stable build --release
cargo +stable test
RUSTFLAGS='-C target-feature=+crt-static' cargo +stable build --release --target x86_64-unknown-linux-musl
```

### 10.2 Packaging Strategy

- Primary: stripped release binaries (`apate-client`, `apate-server`).
- Optional: minimal Docker image for server deployments.

### 10.3 Health Checks

- Client: session state + packet loop health.
- Server: listener health + auth backend readiness + facade path health.

### 10.4 Monitoring

- Structured logs with event codes.
- Metrics: handshake success rate, fallback rate, packet loss recovery rate, rekey events, probe rejects.

## 11. Development Workflow

### 11.1 Local Setup

```bash
rustup toolchain install stable
rustup default stable
cargo fetch
cargo test
```

### 11.2 Code Standards

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Explicit module boundaries and no unchecked unwraps in production path.

### 11.3 Git Workflow

- Short-lived feature branches from `main`.
- PR required with passing CI/security gates.
- Squash merge for linear history.
