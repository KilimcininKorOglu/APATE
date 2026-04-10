# Apate — Specification

> Stealth VPN protocol designed for low-latency tunnel transport and DPI evasion while keeping dependency footprint minimal.

**Source:** Derived from `./prd.md` (read 2026-04-11). Engineering gaps filled via targeted elicitation.

## 1. Overview

### 1.1 What Is Apate?

Apate is a Rust-based stealth VPN protocol and runtime-focused networking system. It aims to provide encrypted tunnel transport that remains operational under restrictive network environments where DPI, active probing, and protocol fingerprinting are present.

Project prioritizes three outcomes equally: low dependency count, low per-packet latency overhead, and high censorship resistance. Product ships as client/server software with protocol-level camouflage and configurable transport behavior for hostile or unstable networks.

### 1.2 Target Audience

- Security and networking engineers operating in censorship-heavy regions
- Infrastructure teams that need hardened tunnel transport without heavyweight runtime stacks
- Advanced self-hosters and operators who require protocol control, not turnkey consumer VPN UX
- Research and red-team environments evaluating DPI-resilient transport channels

### 1.3 Key Differentiators

- Curated minimal dependency policy with strict dependency budget and audit requirements
- Protocol-level stealth behavior targeting indistinguishability from legitimate HTTPS/TLS 1.3 traffic
- Full v1.0 scope includes pluggable auth backends (static key, token, certificate)
- Multi-mode transport strategy with automatic UDP-first and TCP fallback behavior
- Performance-first target profile with microsecond-level protocol overhead goals

### 1.4 Competitive Landscape

| Capability                | Apate | WireGuard | OpenVPN |
|--------------------------|:-----:|:---------:|:-------:|
| Built-in DPI camouflage  |  Yes  |    No     | Limited |
| Active probing defense   |  Yes  |    No     |   No    |
| Transport fallback model |  Yes  |  Partial  |   Yes   |
| Dependency minimization  |  Yes  |  Medium   |   Low   |
| Pluggable auth backends  |  Yes  |  Limited  |   Yes   |

## 2. Core Concepts

| Concept              | Definition |
|---------------------|------------|
| Stealth profile     | Configurable fingerprint and traffic-shaping behavior used to mimic legitimate client traffic patterns |
| Camouflage mode     | Wire-level disguise strategy used for transport packets (TLS-like or QUIC-like presentation) |
| Handshake           | Initial key establishment sequence that authenticates peers and derives transport keys |
| Connection migration| Continuation of tunnel session across client network path/IP changes without full reconnect |
| FEC group           | Packet set used for parity-based loss recovery without immediate retransmission |
| Control channel     | Reserved tunnel stream for protocol control messages (rekey, migrate, close) |
| Auth backend        | Mechanism that validates client identity (static key, token, certificate) |
| Web facade          | Server behavior presented to unauthenticated probes to reduce protocol exposure |

## 3. Functional Requirements

### 3.1 Connection & Session Lifecycle

#### 3.1.1 Session Establishment

**User Story:** As tunnel client, I want fast authenticated session setup so traffic can flow with minimal startup delay.

**Description:** Client and server establish authenticated encrypted session in one round trip for standard flow, with optional resumed flow behavior for reconnect scenarios.

**Acceptance Criteria:**
- [ ] Session establishment succeeds only with valid server identity and client auth backend validation.
- [ ] First protected data transfer occurs after successful key establishment.
- [ ] Reconnection path supports reduced-RTT behavior when prior session state exists.

**Edge Cases:**
- Server key mismatch at client bootstrap.
- Handshake packets dropped or reordered.
- Replay attempt on resumed session bootstrap.

**Constraints:**
- Must support operation on public internet links with variable RTT and loss.
- Must fail closed on authentication or key schedule inconsistencies.

#### 3.1.2 Session Continuity & Rekey

**User Story:** As operator, I want long-lived sessions to rotate keys safely without user-visible tunnel interruption.

**Description:** Active sessions rotate traffic keys by policy triggers and maintain transport continuity.

**Acceptance Criteria:**
- [ ] Rekey policy supports both time-based and data-volume-based triggers.
- [ ] Data channel remains active during rekey transition.
- [ ] Session terminates on key/nonce safety boundary violation.

**Edge Cases:**
- Delayed rekey control message.
- Simultaneous rekey trigger at both peers.
- Counter nearing exhaustion.

**Constraints:**
- No plaintext fallback allowed.

### 3.2 Transport Behavior

#### 3.2.1 Multi-Mode Transport Selection

**User Story:** As client, I want automatic transport fallback so tunnel remains reachable on restrictive networks.

**Description:** Client attempts preferred low-latency transport first and falls back to restrictive-compatible transport after timeout policy.

**Acceptance Criteria:**
- [ ] Default mode attempts UDP-based path first.
- [ ] Fallback to TCP-based path occurs when initial path unavailable within configured timeout.
- [ ] Operator can force transport mode via configuration.

**Edge Cases:**
- Flaky network causing transient UDP availability.
- Middlebox changing behavior during active session.
- Forced mode incompatible with environment.

**Constraints:**
- Control logic must remain deterministic under repeated fallback events.

#### 3.2.2 Reliability, Loss Recovery, and Congestion Handling

**User Story:** As user, I want stable throughput and low jitter under lossy links.

**Description:** Transport provides ACK-based recovery, optional parity recovery, and pacing/congestion behavior tuned for tunnel workloads.

**Acceptance Criteria:**
- [ ] Lost payload recovery works via retransmit and/or parity path based on mode and policy.
- [ ] Congestion adaptation reduces sustained packet loss under load.
- [ ] TCP fallback disables parity recovery to avoid redundant reliability layering.

**Edge Cases:**
- Burst loss periods.
- Persistent high-latency links.
- Path with asymmetrical bandwidth.

**Constraints:**
- Reliability features must not violate packet confidentiality.

### 3.3 Stealth & Censorship Resistance

#### 3.3.1 Traffic Camouflage and Shaping

**User Story:** As censored-network user, I want tunnel traffic to resemble legitimate encrypted traffic so blocking risk decreases.

**Description:** Tunnel packets follow selected stealth profile for structural presentation, size distribution shaping, timing variance, and optional idle masking.

**Acceptance Criteria:**
- [ ] Wire presentation conforms to selected camouflage mode format.
- [ ] Packet size and timing behavior follow configured profile bounds.
- [ ] Profile selection supports built-in defaults and runtime override inputs.

**Edge Cases:**
- Invalid custom profile data.
- Highly variable application burst traffic.
- Idle periods exceeding configured thresholds.

**Constraints:**
- Added stealth overhead must stay within configured latency budget.

#### 3.3.2 Active Probing Defense

**User Story:** As server operator, I want unauthorized probes to see plausible web behavior instead of protocol fingerprint leaks.

**Description:** Server distinguishes authenticated tunnel attempts from unauthenticated probe attempts and serves facade behavior for non-authenticated traffic.

**Acceptance Criteria:**
- [ ] Valid authenticated attempts route to tunnel path.
- [ ] Invalid/unknown attempts route to facade path.
- [ ] Facade path emits protocol-consistent web responses.

**Edge Cases:**
- Repeated probe attempts from rotating addresses.
- Partial handshake probes.
- TLS/session anomalies from scanners.

**Constraints:**
- Probe handling must not expose tunnel-specific error signatures.

### 3.4 Tunnel Data Plane & Routing

#### 3.4.1 Packet Tunnel I/O

**User Story:** As user, I want IP packets to flow through secure tunnel with deterministic handling.

**Description:** Client and server exchange encapsulated IP payloads via tunnel interface and maintain packet integrity end-to-end.

**Acceptance Criteria:**
- [ ] Supported tunnel interfaces can read/write IPv4 and IPv6 payloads.
- [ ] Encapsulation and decapsulation preserve payload integrity.
- [ ] MTU handling avoids silent packet truncation.

**Edge Cases:**
- Oversized packets.
- Fragmentation interaction with network path.
- Interface device restart or reset.

**Constraints:**
- Packet processing path must support continuous high-rate operation.

#### 3.4.2 Routing and DNS Leak Prevention

**User Story:** As privacy-sensitive user, I want DNS and routed traffic to stay inside configured tunnel policy.

**Description:** Routing policy supports full or split mode and enforces DNS handling through secure path by default.

**Acceptance Criteria:**
- [ ] Full-tunnel mode routes all eligible traffic through tunnel.
- [ ] Split mode respects bypass CIDR definitions.
- [ ] DNS resolution follows configured secure mode and does not leak outside policy.

**Edge Cases:**
- Invalid CIDR configuration.
- DNS upstream outage.
- Dual-stack routing conflicts.

**Constraints:**
- Route policy updates must not break active session continuity.

### 3.5 Authentication & Access Control

#### 3.5.1 Pluggable Auth Backends

**User Story:** As operator, I want multiple authentication methods so deployment can match trust model.

**Description:** Server supports static key, token, and certificate backends in v1.0. Deployments may enable one or multiple methods by policy.

**Acceptance Criteria:**
- [ ] Static key backend validates authorized key material.
- [ ] Token backend validates signature and policy constraints.
- [ ] Certificate backend validates presented identity chain and trust anchor.
- [ ] Backend enablement is configurable without code changes.

**Edge Cases:**
- Expired token/certificate.
- Revoked or rotated credentials.
- Backend mismatch between client and server policy.

**Constraints:**
- Authentication failures must produce non-sensitive error outputs.

### 3.6 Operations & Configuration

#### 3.6.1 CLI + Config Driven Operations

**User Story:** As operator, I want deterministic file/CLI driven control without external management API.

**Description:** Runtime is controlled through configuration files and command-line interface only. No REST or gRPC management plane is exposed in v1.0.

**Acceptance Criteria:**
- [ ] Client and server start with explicit config inputs.
- [ ] Runtime mode/auth/profile settings are fully configurable via config.
- [ ] Configuration validation reports actionable errors before session startup.

**Edge Cases:**
- Missing required config keys.
- Unsupported config value combinations.
- Live profile reload failure.

**Constraints:**
- Configuration parser behavior must be deterministic and strict.

## 4. Architecture Overview

### 4.1 System Components

- Client runtime: local tunnel endpoint that manages session setup, transport, and route policy.
- Server runtime: remote tunnel endpoint handling client auth, session management, and egress flow.
- Protocol core: handshake, key lifecycle, framing, and secure payload processing.
- Stealth subsystem: camouflage mode selection, profile-driven shaping, and facade behavior.
- Transport subsystem: mode selection, reliability, pacing, congestion, and migration behavior.
- Tunnel/routing subsystem: interface packet I/O, routing decisions, and DNS policy.
- Configuration subsystem: parsing, validation, profile selection, and runtime loading.

### 4.2 Component Interactions

- Client tunnel I/O emits packet flow to protocol core through transport subsystem.
- Protocol core enforces authenticated encryption and session key lifecycle.
- Stealth subsystem wraps and shapes transport outputs before wire transmission.
- Server reverses flow: accepts wire input, validates/authenticates, decapsulates, routes to egress.
- Configuration governs behavior of all runtime subsystems, including auth, profile, transport, and policy.

### 4.3 External Integrations

| Integration              | Purpose | Fallback Behavior |
|-------------------------|---------|-------------------|
| Network stack (UDP/TCP) | Wire transport path | Mode fallback and reconnect policy |
| Tunnel device interface | Inject/read IP packets | Session startup fails with explicit device error |
| DNS upstream resolvers  | Resolve DNS in protected path | Configured fallback mode if secure upstream unavailable |
| TLS cert/domain facade inputs | Probe-resistant facade behavior | Tunnel path remains gated by auth check |

## 5. Data Model

### 5.1 Core Entities

#### ConnectionSession

| Field             | Type   | Required | Description                            | Constraints |
|------------------|--------|----------|----------------------------------------|-------------|
| connectionId     | Bytes  | Yes      | Stable identifier for tunnel session   | Unique per active session |
| state            | Enum   | Yes      | Session lifecycle state                | Valid state transitions only |
| transportMode    | Enum   | Yes      | Active wire mode                       | Must be configured/negotiated value |
| authMethod       | Enum   | Yes      | Method used for client validation      | One of enabled backends |
| establishedAt    | Time   | Yes      | Session establishment timestamp         | Immutable after set |
| peerEndpoint     | String | Yes      | Current remote address mapping         | Updatable only via migration rules |

#### CryptoContext

| Field               | Type  | Required | Description                           | Constraints |
|--------------------|-------|----------|---------------------------------------|-------------|
| keyEpoch           | U64   | Yes      | Current key generation/version        | Monotonic increasing |
| txNonceCounter     | U64   | Yes      | Outbound nonce counter                | Must not reuse values |
| rxWindowState      | Blob  | Yes      | Replay/loss tracking metadata         | Must align with frame sequence |
| rekeyTimePolicySec | U32   | Yes      | Time-based rekey trigger              | Positive value |
| rekeyBytePolicy    | U64   | Yes      | Byte-volume rekey trigger             | Positive value |

#### StealthProfile

| Field             | Type   | Required | Description                           | Constraints |
|------------------|--------|----------|---------------------------------------|-------------|
| profileName      | String | Yes      | Selected profile identifier           | Must exist in built-in or custom profile source |
| camouflageMode   | Enum   | Yes      | Wire presentation mode                | Supported mode only |
| packetSizePolicy | Object | Yes      | Size shaping parameters               | Within MTU-safe bounds |
| timingPolicy     | Object | Yes      | Inter-packet timing/jitter parameters | Must respect max jitter budget |
| idleMasking      | Bool   | Yes      | Idle padding traffic enablement       | Default false/true per config |

#### AuthPolicy

| Field           | Type   | Required | Description                           | Constraints |
|----------------|--------|----------|---------------------------------------|-------------|
| enabledMethods | Array  | Yes      | Allowed auth backends                 | Non-empty |
| staticKeySet   | Blob   | No       | Authorized static keys                | Required when static key backend enabled |
| tokenVerifier  | Object | No       | Token signature/claims policy         | Required when token backend enabled |
| certificateCa  | Blob   | No       | Trust anchor for certificate backend  | Required when certificate backend enabled |

### 5.2 Relationships

- ConnectionSession → has one → CryptoContext
- ConnectionSession → uses one → StealthProfile
- ConnectionSession → validated by → AuthPolicy
- AuthPolicy → may include → multiple backend-specific verifier definitions

### 5.3 Data Lifecycle

- Session state is created on handshake start and destroyed on disconnect/timeout.
- Cryptographic runtime material is rotated during session and destroyed at session end.
- Config/profile/auth policy loaded at startup, with selective runtime reload for supported profile inputs.
- Operational counters/telemetry are ephemeral unless explicit logging/export policy enabled.

## 6. API Surface

### 6.1 API Style

No external management API in v1.0. Exposed control surface is CLI plus configuration files. Programmatic remote administration endpoints are explicitly out of scope.

### 6.2 Command Surface Overview

| Command Pattern                 | Description                                  | Auth Context |
|--------------------------------|----------------------------------------------|--------------|
| `apate-client --config <path>` | Starts client tunnel session from config     | Local process privileges |
| `apate-server --config <path>` | Starts server listener and auth policy stack | Local process privileges |
| `SIGHUP` (where supported)     | Triggers profile/config reload behavior      | Local signal permission |

### 6.3 Authentication & Authorization

- Runtime startup and control require host-level process permissions.
- Network client access authorization enforced by configured auth backend policy.

### 6.4 Rate Limiting

- Server must support policy controls preventing unbounded unauthenticated probe/connection abuse.
- Exact thresholds are deployment policy values.

### 6.5 Error Format

CLI and logs return structured machine-readable codes with human-readable messages. Sensitive auth/crypto details must never appear in failure output.

## 7. User Interface

### 7.1 Interface Type

CLI and configuration-driven operation.

### 7.2 Key Interaction Surfaces

- Client startup command: load config, initialize tunnel, establish session.
- Server startup command: load config, initialize listeners, apply auth and facade behavior.
- Runtime reload trigger: apply eligible profile/config changes without full restart where supported.

### 7.3 Accessibility & UX Constraints

- Deterministic CLI behavior and explicit error outputs required.
- No graphical UI in v1.0 scope.

## 8. Security Model

### 8.1 Authentication

v1.0 supports three backend types:

- Static key trust model
- Token-based trust model
- Certificate-based trust model

Deployments can enable one or multiple methods.

### 8.2 Authorization

- Authorization policy derived from selected auth backend(s).
- Server accepts tunnel access only when presented identity satisfies enabled backend policy.

### 8.3 Data Protection

- Tunnel payload confidentiality and integrity are mandatory for all transport modes.
- Sensitive key material must be short-lived and cleared on lifecycle end.
- Connection metadata exposure should be minimized in logs.

### 8.4 Input Validation

- Strict validation at configuration load, handshake parsing, frame parsing, and auth token/certificate boundaries.
- Malformed input must fail closed and avoid parser ambiguity paths.

## 9. Deployment Model

### 9.1 Target Environments

- Linux, macOS, and Windows are in v1.0 scope for client/server availability.
- Development and staging environments should mirror production network constraints where possible.

### 9.2 Distribution Method

- Primary distribution model is standalone binaries for client and server processes.
- Containerized deployment may be supported by operators but is not required as core delivery model.

### 9.3 Configuration

- File-based configuration is primary.
- CLI flags select config path and runtime mode.
- Profile override file path is optional; built-in profiles remain default.

### 9.4 System Requirements

- Host OS with tunnel device support and required network privileges.
- Sufficient CPU for cryptographic operations and traffic shaping at target throughput.
- Stable clock/timer behavior for pacing and timeout policies.

## 10. Performance Requirements

### 10.1 Response Time Targets

| Metric                             | Target |
|------------------------------------|--------|
| Session handshake compute budget   | < 400μs (excluding network RTT) |
| Per-packet processing overhead      | < 10μs steady state (excluding path delay) |
| Reconnect fast path compute budget | < 50μs where resume conditions valid |

### 10.2 Throughput Targets

| Scenario                        | Target |
|---------------------------------|--------|
| 1 Gbps environment              | Near line-rate tunnel throughput |
| 10 Gbps environment (multi-core)| High-throughput operation with minimal collapse under loss |
| Lossy WAN links                 | Maintain usable throughput with adaptive recovery |

### 10.3 Resource Limits

| Resource          | Requirement |
|-------------------|-------------|
| Memory usage      | Bounded by configured buffer and connection limits |
| CPU overhead      | Optimized for sustained packet path and cryptographic workload |
| Dependency growth | Must remain within project-defined direct/transitive limits |

## 11. Constraints & Non-Goals

### 11.1 Technical Constraints

- Rust implementation with minimal curated dependency policy.
- No general-purpose async runtime framework dependency.
- Full v1.0 scope includes cross-platform support and all auth backends.
- Control plane is CLI/config only; no external remote management API in v1.0.

### 11.2 Non-Goals

- **Consumer VPN product UX:** Native consumer GUI/app ecosystem is out of scope.
- **Anonymity network semantics:** Onion-routing style anonymity guarantees are out of scope.
- **General web proxy replacement:** Project targets tunnel protocol, not full secure web gateway product.
- **Arbitrary plugin marketplace:** Runtime extensibility by untrusted plugins is out of scope.
- **Remote orchestration API in v1.0:** REST/gRPC management plane excluded.
- **Feature parity with full QUIC/TLS stacks:** Mimicry goals do not imply full RFC-complete protocol stacks.

### 11.3 Assumptions

- Operators can provision required host/network privileges for tunnel interfaces.
- Target environments tolerate required ports and encrypted traffic patterns.
- Selected cryptographic primitives remain secure under current threat assumptions.
- Operators can manage auth material lifecycle (keys/tokens/certificates) safely.

### 11.4 Open Questions

- **Default auth backend precedence:** policy-order
- **Baseline probe-rate defense defaults:** strict
- **Telemetry verbosity baseline:** minimal

## 12. Future Considerations

- **v1.1:** Broader tuning profiles and additional operator tooling for profile validation and diagnostics.
- **v1.1:** Expanded platform hardening and deployment automation templates.
- **v2.0:** Optional managed control-plane integration if operational demand justifies API surface expansion.
