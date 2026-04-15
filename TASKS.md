# Apate — Tasks

> Ordered work breakdown derived from `IMPLEMENTATION.md`.
> Execute sequentially. Each task is completable in one focused session.

## Summary

| Metric              | Value         |
|---------------------|---------------|
| Total Tasks         | 28            |
| Phases              | 4             |
| Estimated Effort    | 180-240 hours |
| Foundation Complete | After Task 5  |
| MVP Complete        | After Task 21 |
| Full Release        | After Task 28 |

---

## Phase 1: Foundation & Core Contracts

> After this phase: project compiles, module skeleton exists, crypto/config/runtime foundations are testable.

### Task 1: Workspace Scaffolding and Tooling

**Files to create/modify:**
- `Cargo.toml`
- `.cargo/config.toml`
- `build.rs`
- `DEPS.md`
- `.github/workflows/ci.yml`
- `src/lib.rs`
- `src/main.rs`
- Directory skeleton from `IMPLEMENTATION.md` §3.1

**Description:**
Create Rust workspace baseline, dependency set, lint/format/test scripts, and top-level crate wiring.

**Code requirements:**
- Add curated dependency set from `IMPLEMENTATION.md` §1.3.
- Configure strict lint profile and release profile flags.
- Add feature flags for platform backends.

**Test requirements:**
- `cargo +stable check`
- `cargo +stable test`

**Acceptance Criteria:**
- [ ] Build and test commands pass.
- [ ] Dependency list matches curated inventory.
- [ ] All planned top-level directories exist.

**Dependencies:** None  
**Effort:** 3-4 hours  
**Refs:** IMPLEMENTATION.md §1, §3.1

### Task 2: Domain Types and Error Taxonomy

**Files to create/modify:**
- `src/util/mod.rs`
- `src/auth/mod.rs`
- `src/transport/mod.rs`
- `src/noise/mod.rs`
- `src/config/types.rs`
- `src/telemetry/mod.rs`

**Description:**
Define shared enums/structs for sessions, auth, transport, stealth, and error categories.

**Code requirements:**
- Add typed state and message enums.
- Add module-level error enums mapped to global error taxonomy.
- Ensure no `unwrap` usage in domain path.

**Test requirements:**
- Unit tests for state/error serialization assumptions.

**Acceptance Criteria:**
- [ ] All core entities from SPEC §5 represented in types.
- [ ] Error taxonomy aligns with IMPLEMENTATION §6.1.
- [ ] `cargo +stable check` passes with no warnings.

**Dependencies:** Task 1  
**Effort:** 3-4 hours  
**Refs:** SPECIFICATION.md §5; IMPLEMENTATION.md §6

### Task 3: Config Loader and Parser Baseline

**Files to create/modify:**
- `src/config/mod.rs`
- `src/config/parser.rs`
- `src/config/types.rs`
- `src/config/profiles/mod.rs`
- `tests/property/parser_fuzz_harness.rs`

**Description:**
Implement strict TOML-subset parser and validated runtime config loader.

**Code requirements:**
- Parser must reject unknown required sections and malformed values.
- Add config source precedence: defaults → file → CLI override.
- Add profile selector contract.

**Test requirements:**
- Unit tests for valid/invalid parsing.
- Property tests for parser robustness.

**Acceptance Criteria:**
- [ ] Valid config files parse into typed `AppConfig`.
- [ ] Invalid configs fail with deterministic error codes.
- [ ] Parser tests pass in CI.

**Dependencies:** Task 2  
**Effort:** 4-6 hours  
**Refs:** SPECIFICATION.md §3.6, §9; IMPLEMENTATION.md §7

### Task 4: Runtime Reactor and Timer Skeleton

**Files to create/modify:**
- `src/runtime/mod.rs`
- `src/runtime/reactor.rs`
- `src/runtime/executor.rs`
- `src/runtime/timer.rs`
- `src/runtime/waker.rs`
- `src/runtime/backend/mod.rs`

**Description:**
Create runtime interfaces and platform backend abstraction with no-op/smoke backend behavior.

**Code requirements:**
- Define backend trait and event dispatch contract.
- Implement timer wheel skeleton and task queue interface.
- Provide compile-gated backend module wiring.

**Test requirements:**
- Unit tests for timer scheduling behavior.
- Smoke test for reactor event loop bootstrap.

**Acceptance Criteria:**
- [ ] Runtime loop starts/stops cleanly in tests.
- [ ] Timer scheduling API behaves deterministically.
- [ ] Backend abstraction compiles on host platform.

**Dependencies:** Task 2  
**Effort:** 5-6 hours  
**Refs:** SPECIFICATION.md §4; IMPLEMENTATION.md §3.2

### Task 5: Crypto Wrappers and Key Lifecycle Primitives

**Files to create/modify:**
- `src/crypto/mod.rs`
- `src/crypto/aead.rs`
- `src/crypto/kx.rs`
- `src/crypto/sign.rs`
- `src/crypto/kdf.rs`
- `src/crypto/rng.rs`
- `tests/integration/handshake.rs`

**Description:**
Implement AEAD wrappers, hybrid key exchange scaffolding, KDF interface, and zeroization-safe key handling.

**Code requirements:**
- Unified cipher enum and in-place encrypt/decrypt API.
- Hybrid KX output combiner contract.
- Key zeroization wrappers for secret structs.

**Test requirements:**
- Known-vector tests for AEAD/KDF wrappers.
- Integration smoke test for key derivation flow.

**Acceptance Criteria:**
- [ ] Crypto wrappers pass vector tests.
- [ ] Secret data types implement zeroize semantics.
- [ ] `cargo +stable test` passes for crypto module.

**Dependencies:** Task 1, Task 2  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.1, §8; IMPLEMENTATION.md §1.3

---

## Phase 2: Protocol Core and Data Plane

> After this phase: tunnel core works in baseline mode with framing, routing, and transport lifecycle.

### Task 6: Noise Handshake State Machine

**Files to create/modify:**
- `src/noise/handshake.rs`
- `src/noise/state.rs`
- `src/noise/cipher_state.rs`
- `src/noise/symmetric_state.rs`
- `tests/integration/handshake.rs`

**Description:**
Implement handshake and transition to transport keys using explicit state machine transitions.

**Code requirements:**
- Enforce valid transition graph only.
- Bind auth validation hook into handshake completion.

**Test requirements:**
- Handshake success and failure path tests.
- Replay and invalid-state transition tests.

**Acceptance Criteria:**
- [ ] Handshake completes to `Established` state on valid input.
- [ ] Invalid transition attempts fail deterministically.
- [ ] Handshake tests cover success/failure/replay scenarios.

**Dependencies:** Task 5  
**Effort:** 5-7 hours  
**Refs:** SPECIFICATION.md §3.1; IMPLEMENTATION.md §2.4

### Task 7: Frame Codec and Packet Pipeline Contracts

**Files to create/modify:**
- `src/transport/frame.rs`
- `src/transport/mod.rs`
- `src/util/buf.rs`
- `tests/property/frame_roundtrip.rs`

**Description:**
Build frame encode/decode logic and packet context structures for transport pipeline.

**Code requirements:**
- Implement binary header parse/serialize.
- Validate length, type, and flags strictly.
- Expose packet context type for stage pipeline.

**Test requirements:**
- Property tests for roundtrip encode/decode.
- Negative tests for malformed headers.

**Acceptance Criteria:**
- [ ] Roundtrip tests pass for all frame types.
- [ ] Malformed frames are rejected without panic.
- [ ] Frame parser alloc behavior stays bounded.

**Dependencies:** Task 2, Task 5  
**Effort:** 4-6 hours  
**Refs:** SPECIFICATION.md §3.2, §5.2; IMPLEMENTATION.md §2.5

### Task 8: Transport Engine and Mode Negotiation

**Files to create/modify:**
- `src/transport/connection.rs`
- `src/transport/mode.rs`
- `src/transport/udp_tls.rs`
- `src/transport/tcp_tls.rs`
- `src/transport/quic_mask.rs`
- `tests/integration/fallback.rs`

**Description:**
Implement transport strategy selection (`auto`, forced mode), connection lifecycle, and fallback behavior.

**Code requirements:**
- Strategy pattern implementation from IMPLEMENTATION §2.3.
- Deterministic UDP-first timeout fallback to TCP.
- Connection lifecycle tied to state machine events.

**Test requirements:**
- Integration tests for fallback conditions.
- Unit tests for mode selection logic.

**Acceptance Criteria:**
- [ ] `auto` mode falls back per configured timeout.
- [ ] Forced mode bypasses negotiation path.
- [ ] Fallback tests pass under simulated failure.

**Dependencies:** Task 6, Task 7  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.2; IMPLEMENTATION.md §5.2

### Task 9: ACK, Loss Recovery, Congestion, and Pacing

**Files to create/modify:**
- `src/transport/ack.rs`
- `src/transport/loss.rs`
- `src/transport/congestion.rs`
- `src/transport/pacing.rs`
- `tests/integration/tunnel_data.rs`

**Description:**
Add reliability and pacing behavior for steady-state transport.

**Code requirements:**
- Selective ACK tracking and loss detection.
- Congestion state machine and pacing scheduler hooks.
- Recovery policy compatible with transport modes.

**Test requirements:**
- Loss injection integration tests.
- Unit tests for ack bitmask and timing math.

**Acceptance Criteria:**
- [ ] Retransmit path recovers dropped frames.
- [ ] Pacing delays remain bounded under load simulation.
- [ ] Congestion transitions are observable in metrics/logs.

**Dependencies:** Task 8  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.2; IMPLEMENTATION.md §3.2

### Task 10: Tunnel Device Adapters (Linux + macOS)

**Files to create/modify:**
- `src/tunnel/mod.rs`
- `src/tunnel/packet.rs`
- `src/tunnel/tun_linux.rs`
- `src/tunnel/tun_macos.rs`
- `tests/integration/tunnel_data.rs`

**Description:**
Implement Linux and macOS tunnel adapters and packet read/write loop integration.

**Code requirements:**
- Adapter trait from ports-and-adapters pattern.
- Platform-specific open/configure/read/write contracts.
- Unified packet abstraction for transport pipeline input.

**Test requirements:**
- Adapter interface unit tests with loopback harness.
- Integration smoke test for packet path.

**Acceptance Criteria:**
- [ ] Linux adapter initializes and exchanges packets in test harness.
- [ ] macOS adapter compiles and passes interface tests.
- [ ] Packet parser validates IPv4/IPv6 boundaries.

**Dependencies:** Task 4, Task 7  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.4; IMPLEMENTATION.md §3.2

### Task 11: Tunnel Adapters (Windows + FreeBSD)

**Files to create/modify:**
- `src/tunnel/tun_windows.rs`
- `src/tunnel/tun_freebsd.rs`
- `src/runtime/backend/iocp.rs`
- `src/runtime/backend/kqueue.rs`

**Description:**
Add Windows and FreeBSD adapter implementations plus backend integration stubs.

**Code requirements:**
- Compile-gated platform modules.
- Shared adapter contract compliance.

**Test requirements:**
- Cross-target compile checks.
- Host-independent interface tests.

**Acceptance Criteria:**
- [ ] Windows and FreeBSD modules compile with target-specific features.
- [ ] Adapter trait conformance tests pass.
- [ ] No unsupported target code leaks into default build.

**Dependencies:** Task 10  
**Effort:** 4-6 hours  
**Refs:** SPECIFICATION.md §9.1; IMPLEMENTATION.md §3.1

### Task 12: Routing Engine and DNS Policy

**Files to create/modify:**
- `src/routing/mod.rs`
- `src/routing/table.rs`
- `src/routing/split.rs`
- `src/routing/dns.rs`
- `src/routing/doh.rs`
- `tests/integration/tunnel_data.rs`

**Description:**
Implement route lookup, full/split policy, DNS interception, and DoH forwarding path.

**Code requirements:**
- Route table with deterministic lookup.
- DNS mode selection (`doh`, `plain`, fallback mode).
- Leak-prevention policy in routing path.

**Test requirements:**
- Route lookup tests for CIDR match precedence.
- DNS mode integration tests.

**Acceptance Criteria:**
- [ ] Full and split routing policies behave as configured.
- [ ] DNS requests follow configured protected path.
- [ ] Route/DNS tests pass with dual-stack cases.

**Dependencies:** Task 7, Task 10  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.4; IMPLEMENTATION.md §4

---

## Phase 3: Stealth, Authentication, and Session Resilience

> After this phase: full v1.0 functional scope achieved (MVP milestone).

### Task 13: Stealth Profile Runtime and Loader

**Files to create/modify:**
- `src/config/profiles/mod.rs`
- `src/config/profiles/chrome_131.rs`
- `src/config/profiles/firefox_130.rs`
- `src/config/profiles/safari_18.rs`
- `src/stealth/mod.rs`

**Description:**
Implement profile model, built-in profile registry, and override loading from file.

**Code requirements:**
- Profile validation for packet/timing bounds.
- Runtime profile selection by name/path.

**Test requirements:**
- Unit tests for profile selection and invalid profile rejection.

**Acceptance Criteria:**
- [ ] Built-in profiles load by name.
- [ ] File-based profile overrides validate strictly.
- [ ] Invalid profiles fail without runtime panic.

**Dependencies:** Task 3  
**Effort:** 4-5 hours  
**Refs:** SPECIFICATION.md §3.3; IMPLEMENTATION.md §7

### Task 14: TLS Camouflage and Packet Shaping

**Files to create/modify:**
- `src/stealth/tls_camouflage.rs`
- `src/stealth/client_hello.rs`
- `src/stealth/server_hello.rs`
- `src/stealth/padding.rs`
- `src/stealth/timing.rs`

**Description:**
Implement TLS-like wrapping, handshake mimic components, packet size shaping, and timing jitter stages.

**Code requirements:**
- Pipeline stage composition from IMPLEMENTATION §2.5.
- Bound jitter and size shaping by selected profile parameters.

**Test requirements:**
- Unit tests for record/header structure.
- Statistical tests for packet size and timing bounds.

**Acceptance Criteria:**
- [ ] Wrapped packets conform to expected record format.
- [ ] Shaping outputs respect configured min/max bounds.
- [ ] Statistical checks pass against selected profile thresholds.

**Dependencies:** Task 7, Task 13  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.3; IMPLEMENTATION.md §2.5

### Task 15: QUIC-Mask Camouflage Path

**Files to create/modify:**
- `src/stealth/quic_camouflage.rs`
- `src/transport/quic_mask.rs`
- `tests/integration/fallback.rs`

**Description:**
Implement QUIC-like packet presentation mode integrated with transport strategy layer.

**Code requirements:**
- QUIC-like header formatter and parser.
- Mode handoff with deterministic fallback behavior.

**Test requirements:**
- Unit tests for header format fields.
- Integration tests for quic_mask mode negotiation.

**Acceptance Criteria:**
- [ ] QUIC-mask mode can send and receive tunnel data in tests.
- [ ] Header parse/serialize roundtrips pass.
- [ ] Mode switching logic remains deterministic.

**Dependencies:** Task 8, Task 14  
**Effort:** 4-6 hours  
**Refs:** SPECIFICATION.md §3.2, §3.3; IMPLEMENTATION.md §5.2

### Task 16: Active Probing Defense and Web Facade

**Files to create/modify:**
- `src/stealth/facade.rs`
- `src/auth/mod.rs`
- `src/config/types.rs`
- `tests/integration/handshake.rs`

**Description:**
Implement probe detection branch and facade response path for unauthenticated traffic.

**Code requirements:**
- Auth gate before tunnel admission.
- Facade path returns web-like response flow.

**Test requirements:**
- Integration tests: valid auth routes to tunnel, invalid routes to facade.

**Acceptance Criteria:**
- [ ] Unauthenticated probes do not reach tunnel path.
- [ ] Authenticated flows bypass facade and establish session.
- [ ] No tunnel-specific errors leak in facade path.

**Dependencies:** Task 6, Task 14  
**Effort:** 4-6 hours  
**Refs:** SPECIFICATION.md §3.3.2; IMPLEMENTATION.md §9

### Task 17: Authentication Coordinator + Static Key Backend

**Files to create/modify:**
- `src/auth/backend.rs`
- `src/auth/static_key.rs`
- `src/auth/mod.rs`
- `tests/integration/handshake.rs`

**Description:**
Introduce auth coordinator and static key backend as first production backend.

**Code requirements:**
- Ports-and-adapters trait design from IMPLEMENTATION §2.2.
- Config-driven backend enablement.

**Test requirements:**
- Unit tests for key matching/rejection.
- Integration tests with handshake gating.

**Acceptance Criteria:**
- [ ] Static key backend accepts authorized keys only.
- [ ] Coordinator dispatches backend by config.
- [ ] Auth rejection path returns sanitized error.

**Dependencies:** Task 6, Task 3  
**Effort:** 4-5 hours  
**Refs:** SPECIFICATION.md §3.5; IMPLEMENTATION.md §2.2

### Task 18: Token Authentication Backend

**Files to create/modify:**
- `src/auth/token.rs`
- `src/auth/backend.rs`
- `tests/integration/handshake.rs`

**Description:**
Add token backend including signature verification and claim validation policy.

**Code requirements:**
- Verify signature and policy fields (expiry, audience/issuer if configured).
- Integrate with auth coordinator without backend-specific branching leak.

**Test requirements:**
- Unit tests for valid/expired/invalid signature tokens.
- Integration tests for mixed-backend configuration.

**Acceptance Criteria:**
- [ ] Valid token authenticates successfully.
- [ ] Invalid or expired token rejected with deterministic code.
- [ ] Mixed backend policy works with coordinator.

**Dependencies:** Task 17  
**Effort:** 3-5 hours  
**Refs:** SPECIFICATION.md §3.5; IMPLEMENTATION.md §3.2

### Task 19: Certificate Authentication Backend

**Files to create/modify:**
- `src/auth/certificate.rs`
- `src/auth/backend.rs`
- `tests/fixtures/certs/*`
- `tests/integration/handshake.rs`

**Description:**
Implement certificate-based auth with configurable CA trust anchor.

**Code requirements:**
- Certificate parsing and chain validation contract.
- Coordinator integration and policy-based backend ordering.

**Test requirements:**
- Unit tests for valid chain, expired cert, wrong CA.
- Integration tests for certificate-auth handshake path.

**Acceptance Criteria:**
- [ ] Valid chain authenticates when certificate backend enabled.
- [ ] Invalid chain rejected with sanitized error.
- [ ] Backend ordering and combination rules pass tests.

**Dependencies:** Task 17  
**Effort:** 5-7 hours  
**Refs:** SPECIFICATION.md §3.5; IMPLEMENTATION.md §2.2

### Task 20: Rekey and Connection Migration

**Files to create/modify:**
- `src/noise/state.rs`
- `src/transport/migration.rs`
- `src/transport/connection.rs`
- `tests/integration/migration.rs`

**Description:**
Implement session rekey transitions and endpoint migration without full reconnect.

**Code requirements:**
- State-machine-safe rekey transition path.
- Migration proof validation and endpoint update.

**Test requirements:**
- Integration tests for migration continuity.
- Rekey tests for ongoing data transfer during key rotation.

**Acceptance Criteria:**
- [ ] Rekey occurs without dropping established session traffic.
- [ ] Migration updates endpoint and preserves session.
- [ ] Invalid migration proofs are rejected.

**Dependencies:** Task 6, Task 8  
**Effort:** 5-7 hours  
**Refs:** SPECIFICATION.md §3.1.2; IMPLEMENTATION.md §2.4

### Task 21: FEC, Compression, and Adaptive Recovery

**Files to create/modify:**
- `src/transport/fec.rs`
- `src/transport/connection.rs`
- `src/transport/pacing.rs`
- `tests/integration/tunnel_data.rs`

**Description:**
Add adaptive parity strategy and optional compression policy integration in packet path.

**Code requirements:**
- FEC mode toggles by observed loss policy.
- Ensure disabled FEC behavior in TCP mode.
- Compression stage guarded by entropy/size policy.

**Test requirements:**
- Loss simulation tests for parity recovery.
- Mode tests ensuring TCP path bypasses FEC.

**Acceptance Criteria:**
- [ ] Adaptive FEC policy changes under loss thresholds.
- [ ] TCP mode does not apply parity logic.
- [ ] End-to-end data integrity passes under simulated loss.

**Dependencies:** Task 9, Task 8  
**Effort:** 6-8 hours  
**Refs:** SPECIFICATION.md §3.2; IMPLEMENTATION.md §5.2

---

## Phase 4: Verification, Hardening, and Release

> After this phase: release-ready artifact with security and CI gates.

### Task 22: End-to-End Integration Suite

**Files to create/modify:**
- `tests/integration/handshake.rs`
- `tests/integration/tunnel_data.rs`
- `tests/integration/fallback.rs`
- `tests/integration/migration.rs`

**Description:**
Expand and stabilize full integration suite across handshake, data, fallback, auth, migration flows.

**Code requirements:**
- Cover all enabled auth backends and transport modes.
- Cover profile selection and config permutations.

**Test requirements:**
- `cargo +stable test --test '*'`

**Acceptance Criteria:**
- [ ] Integration suite covers all core flows from SPEC §3.
- [ ] Tests pass deterministically in CI.
- [ ] Failure outputs include actionable diagnostics.

**Dependencies:** Task 21  
**Effort:** 5-7 hours  
**Refs:** SPECIFICATION.md §3; IMPLEMENTATION.md §8

### Task 23: Fuzzing Targets and Crash Triage Harness

**Files to create/modify:**
- `fuzz/Cargo.toml`
- `fuzz/fuzz_targets/frame_parser.rs`
- `fuzz/fuzz_targets/config_parser.rs`
- `fuzz/fuzz_targets/handshake.rs`

**Description:**
Create fuzz targets for untrusted input boundaries and wire parsing logic.

**Code requirements:**
- Deterministic corpus seed directories.
- Crash minimization and reproducible repro script docs in comments.

**Test requirements:**
- Run short fuzz smoke in CI.

**Acceptance Criteria:**
- [ ] Fuzz targets compile and execute.
- [ ] CI includes fuzz smoke stage.
- [ ] Crashes produce reproducible artifacts.

**Dependencies:** Task 7, Task 12, Task 6  
**Effort:** 4-6 hours  
**Refs:** IMPLEMENTATION.md §8, §9

### Task 24: Benchmark Suite and Performance Gates

**Files to create/modify:**
- `benches/packet_path.rs`
- `benches/crypto_wrappers.rs`
- `benches/handshake.rs`
- `Cargo.toml`

**Description:**
Add microbenchmarks and threshold checks for key latency and throughput metrics.

**Code requirements:**
- Criterion benchmarks for packet path and handshake compute.
- Baseline metric output format for CI artifact retention.

**Test requirements:**
- `cargo +stable bench --bench packet_path` (and others)

**Acceptance Criteria:**
- [ ] Benchmarks run without harness failures.
- [ ] Baseline metrics generated for packet and handshake paths.
- [ ] Performance regressions detectable via threshold script.

**Dependencies:** Task 21  
**Effort:** 4-5 hours  
**Refs:** SPECIFICATION.md §10; IMPLEMENTATION.md §8

### Task 25: Security Gates and Dependency Policy Enforcement

**Files to create/modify:**
- `deny.toml`
- `.github/workflows/security.yml`
- `DEPS.md`
- `src/crypto/*` (only if hardening fixes needed)

**Description:**
Enforce `cargo audit` and `cargo deny` policies, add checks for dependency/license constraints.

**Code requirements:**
- Configure deny rules for duplicate versions and restricted licenses.
- Fail CI on advisory findings.

**Test requirements:**
- Run `cargo audit`
- Run `cargo deny check`

**Acceptance Criteria:**
- [ ] Security workflow fails on advisories/license violations.
- [ ] Dependency constraints enforce curated policy limits.
- [ ] `DEPS.md` matches current lockfile decisions.

**Dependencies:** Task 1  
**Effort:** 3-4 hours  
**Refs:** SPECIFICATION.md §1.3, §8; IMPLEMENTATION.md §1.3

### Task 26: Cross-Platform Build Matrix and Release Workflow

**Files to create/modify:**
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `.github/workflows/security.yml`

**Description:**
Configure CI matrix builds and release artifact pipeline for Linux/macOS/Windows targets.

**Code requirements:**
- Build matrix for stable toolchain and target triples.
- Artifact naming and checksum generation.

**Test requirements:**
- Validate workflow syntax and dry-run build jobs.

**Acceptance Criteria:**
- [ ] CI matrix covers all v1.0 platforms.
- [ ] Release workflow packages binaries with checksums.
- [ ] Failed platform build blocks release promotion.

**Dependencies:** Task 11, Task 22  
**Effort:** 4-6 hours  
**Refs:** SPECIFICATION.md §9.1; IMPLEMENTATION.md §10

### Task 27: Packaging, Health Probes, and Operational Metrics

**Files to create/modify:**
- `src/telemetry/log.rs`
- `src/telemetry/metrics.rs`
- `src/main.rs`
- `Dockerfile` (optional)
- `docs/operations.md`

**Description:**
Finalize runtime health reporting, structured metrics, and packaging artifacts.

**Code requirements:**
- Add startup and runtime health probe outputs.
- Add structured event codes for major state changes.

**Test requirements:**
- Integration checks for health output behavior.

**Acceptance Criteria:**
- [ ] Health signals available for client and server runtime.
- [ ] Metrics include handshake/fallback/loss/auth counters.
- [ ] Release artifact includes minimal operational defaults.

**Dependencies:** Task 22, Task 24  
**Effort:** 4-5 hours  
**Refs:** IMPLEMENTATION.md §10

### Task 28: Final Release Validation and Sign-off

**Files to create/modify:**
- `Cargo.toml`
- `Cargo.lock`
- `.github/workflows/*` (only if final gate fix required)

**Description:**
Run full validation suite and finalize v1.0 release candidate.

**Code requirements:**
- No feature changes unless release blockers discovered.
- Tag-ready build metadata and version consistency checks.

**Test requirements:**
- Full CI-equivalent local run:
  - `cargo +stable fmt --all --check`
  - `cargo +stable clippy --all-targets --all-features -- -D warnings`
  - `cargo +stable test`
  - `cargo audit`
  - `cargo deny check`

**Acceptance Criteria:**
- [ ] All validation commands pass.
- [ ] Release artifacts build successfully for target matrix.
- [ ] v1.0 candidate is shippable with no blocker defects.

**Dependencies:** Task 23, Task 24, Task 25, Task 26, Task 27  
**Effort:** 3-4 hours  
**Refs:** SPECIFICATION.md §10, §11; IMPLEMENTATION.md §8, §10

---

## Milestones

| Milestone   | After Task | What’s Achieved                                       | Demo-able? |
|-------------|------------|-------------------------------------------------------|------------|
| Foundation  | Task 5     | Buildable project with config/runtime/crypto basics   | Yes        |
| Core Tunnel | Task 12    | Baseline tunnel + routing + DNS policy                | Yes        |
| MVP         | Task 21    | Full functional scope including stealth + all auth    | Yes        |
| Release RC  | Task 28    | Hardened, validated, cross-platform release candidate | Yes        |

## Dependency Graph

```text
[T1] → [T2] → [T3]
  └──→ [T5] → [T6] → [T7] → [T8] → [T9] → [T21] → [T22] → [T28]
                └──────────────→ [T12] ───────────┘
                    └→ [T10] → [T11] → [T26] ─────┘
[T13] → [T14] → [T15]
              └→ [T16]
[T17] → [T18]
   └──→ [T19]
[T20] ───────────────→ [T22]
[T23] ───────────────→ [T28]
[T24] ────────┬──────→ [T27] → [T28]
[T25] ────────┘
```
