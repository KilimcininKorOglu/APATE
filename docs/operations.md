# Apate Operations Guide

This document covers installation, configuration, execution, key management,
monitoring, log interpretation, and troubleshooting for the Apate stealth VPN
protocol daemon.

---

## 1. Installation

### 1.1 Building from Source

Prerequisites:

- Rust toolchain 1.80 or later (`rustup toolchain install stable`)
- A C linker (`cc`) for linking against `libc` on POSIX targets
- On Windows: the MSVC toolchain or a MinGW cross-compiler

```sh
git clone <repository-url>
cd apate
cargo build --release
```

The compiled binary is placed at `target/release/apate`.

To cross-compile for a specific target:

```sh
# Linux musl (static, no glibc dependency)
cargo build --release --target x86_64-unknown-linux-musl

# Windows from Linux
cargo build --release --target x86_64-pc-windows-gnu
```

### 1.2 Installing the Binary

Copy the binary to a directory on `PATH`. On Linux and macOS:

```sh
install -m 0755 target/release/apate /usr/local/bin/apate
```

On Windows, copy `target\release\apate.exe` to a directory included in
`%PATH%`.

### 1.3 Runtime Privileges

**Linux:** Opening `/dev/net/tun` and configuring the TUN interface requires
either `CAP_NET_ADMIN` or root. Grant the capability to avoid running as root:

```sh
setcap cap_net_admin=ep /usr/local/bin/apate
```

**macOS:** TUN access via `/dev/tun*` requires root or membership in the
`network` group. No additional capability mechanism is available.

**Windows:** WinTUN driver installation requires administrator elevation. Once
the driver is installed the daemon may run without elevation.

**FreeBSD:** Same as macOS; root or appropriate group membership for
`/dev/tun*`.

---

## 2. Configuration

The default configuration file path is `/etc/apate/apate.conf`. An alternate
path may be provided with `-c` / `--config`. The file is loaded by
`config::parser::parse_config`, validated by `AppConfig::validate`, and must
be present before the daemon starts.

### 2.1 File Format

The config file uses a flat `key = value` format. Lines beginning with `#`
are comments. Blank lines are ignored. Values may be quoted with double quotes
or unquoted; leading and trailing whitespace is stripped.

```
# Example configuration
client.server = "203.0.113.1:443"
transport.mode = "auto"
auth.methods   = ["static_key"]
```

Only the keys listed in the table below are accepted. Any unknown key causes
`ConfigError::UnsupportedKey` and the daemon exits with status 1.

### 2.2 Configuration Reference

| Key                           | Required   | Default       | Valid Values                              | Description                                                                                       |
|-------------------------------|------------|---------------|-------------------------------------------|---------------------------------------------------------------------------------------------------|
| `client.server`               | Client     | (none)        | `<host>:<port>` string                    | Server endpoint the client connects to. Required in client mode; validated to be non-empty.        |
| `transport.mode`              | No         | `auto`        | `auto`, `udp`, `tcp`, `quic_mask`         | Transport selection strategy. `auto` attempts UDP-TLS first, falls back to TCP-TLS on timeout or failure. |
| `transport.fallback_timeout`  | No         | `3`           | Integer >= 1                              | Seconds to wait before the `auto` mode triggers a transport fallback. `0` is rejected as invalid. |
| `stealth.profile`             | No         | `chrome_131`  | `chrome_131`, `firefox_130`, `safari_18`  | Built-in stealth profile controlling ALPN, packet size range, and inter-packet jitter.             |
| `stealth.profile_path`        | No         | (empty)       | Filesystem path string                    | Path to a custom profile file that overrides the built-in profile. Takes priority when set.        |
| `auth.methods`                | Server     | (none)        | `["static_key"]`, `["token"]`, `["certificate"]`, or combinations | Ordered list of authentication backends enabled on the server. Required in server mode. |
| `crypto.post_quantum`         | No         | `true`        | `true`, `false`                           | When `true`, the handshake uses hybrid X25519 + ML-KEM key exchange. When `false`, X25519 only.   |
| `crypto.rekey_interval_secs`  | No         | `60`          | u32 >= 1                                  | Time-based rekey trigger: initiate REKEY after this many seconds since last key rotation.          |
| `crypto.rekey_interval_bytes` | No         | `1073741824`  | u64 >= 1                                  | Byte-based rekey trigger: initiate REKEY after this many bytes of data transmitted under the current key. |
| `routing.mode`                | No         | `full`        | `full`, `split`                           | `full` sends all IP traffic through the tunnel. `split` uses the route table; only CIDRs added to the route table are tunneled. |
| `dns.mode`                    | No         | `doh`         | `doh`, `plain`, `fallback`                | DNS resolution strategy. `doh` sends all DNS over HTTPS through the tunnel. `plain` uses the system resolver. `fallback` uses DoH with a system-resolver fallback. |

### 2.3 Authentication Method List Syntax

The `auth.methods` value is a JSON-like bracketed comma-separated list of
quoted method names:

```
auth.methods = ["static_key", "token"]
```

Valid method tokens are `static_key`, `token`, and `certificate`. The order
controls the order in which `AuthCoordinator` searches backends for a given
`AuthMethod`. An empty list (`[]`) is accepted by the parser but will be
rejected by `AppConfig::validate` in server mode (at least one method is
required).

### 2.4 Stealth Profile File Format

A custom stealth profile file (pointed to by `stealth.profile_path`) uses the
same `key = value` format as the main config. Supported profile keys:

| Key               | Type    | Description                                                   |
|-------------------|---------|---------------------------------------------------------------|
| `name`            | string  | Logical profile name, reflected in startup log output.        |
| `alpn`            | string  | ALPN protocol string injected into TLS ClientHello (`h2`, `h3`, etc.). |
| `min_packet_size` | u16     | Minimum padded packet size in bytes. Range: 1-65535.          |
| `max_packet_size` | u16     | Maximum padded packet size in bytes. Must be >= `min_packet_size` and <= 65535. |
| `min_jitter_ms`   | u16     | Minimum inter-packet delay jitter in milliseconds. Range: 0-500. |
| `max_jitter_ms`   | u16     | Maximum inter-packet delay jitter in milliseconds. Must be >= `min_jitter_ms` and <= 500. |

Example custom profile:

```
name            = "custom_h3"
alpn            = "h3"
min_packet_size = 920
max_packet_size = 1200
min_jitter_ms   = 3
max_jitter_ms   = 10
```

### 2.5 Built-in Profile Parameters

| Profile      | ALPN | Min Packet | Max Packet | Min Jitter | Max Jitter |
|--------------|------|------------|------------|------------|------------|
| `chrome_131` | `h2` | 900 bytes  | 1350 bytes | 4 ms       | 18 ms      |
| `firefox_130`| (see source) | (see source) | (see source) | (see source) | (see source) |
| `safari_18`  | (see source) | (see source) | (see source) | (see source) | (see source) |

The authoritative values for `firefox_130` and `safari_18` are in
`src/config/profiles/firefox_130.rs` and `src/config/profiles/safari_18.rs`.

---

## 3. Running Apate

### 3.1 Subcommands

```
apate <COMMAND> [OPTIONS]

COMMANDS:
    client      Connect to a server in client mode
    server      Listen for client connections in server mode
    gen-key     Generate a new X25519 keypair and print the public key
    version     Print the binary version string
    help        Print usage information

OPTIONS:
    -c, --config <PATH>    Config file path (default: /etc/apate/apate.conf)
    -v, --verbose          Enable verbose (debug-level) log output
    -h, --help             Print help
    -V, --version          Print version
```

### 3.2 Client Mode

Start Apate as a VPN client connecting to a remote server. The configuration
file must include `client.server`.

```sh
apate client --config /etc/apate/client.conf
```

On startup the daemon logs:

```
event code=startup detail=mode=client server=203.0.113.1:443 transport=auto
```

### 3.3 Server Mode

Start Apate as a VPN server accepting client connections. The configuration
file must include `auth.methods` with at least one entry.

```sh
apate server --config /etc/apate/server.conf
```

On startup the daemon logs:

```
event code=startup detail=mode=server auth=[static_key]
```

### 3.4 Running as a System Service

**Linux (systemd):**

Create `/etc/systemd/system/apate.service`:

```ini
[Unit]
Description=Apate stealth VPN
After=network.target

[Service]
ExecStart=/usr/local/bin/apate client --config /etc/apate/apate.conf
Restart=on-failure
RestartSec=5s
AmbientCapabilities=CAP_NET_ADMIN
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true

[Install]
WantedBy=multi-user.target
```

Enable and start:

```sh
systemctl daemon-reload
systemctl enable --now apate
```

---

## 4. Key Generation

### 4.1 Generating an X25519 Keypair

The `gen-key` subcommand uses `crypto::rng::os_seed` to draw 32 bytes from
the operating system's CSPRNG, then derives the X25519 public key via
`crypto::kx::derive_public_key`. The 32-byte secret is used for Diffie-
Hellman and must be kept confidential.

```sh
apate gen-key
```

Output:

```
public_key=3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29
```

The 32-byte secret seed used internally to derive this public key must be
stored separately; `gen-key` does not persist it. For production deployments,
generate and store the secret key as a 64-character hex string and load it
into `StaticKeyBackend` as a pre-shared key, or use it as the signing key for
`TokenBackend`.

### 4.2 Generating Ed25519 Signing Keys

Ed25519 keys are used by `auth::common_crypto` for signing `AuthProof`
messages and for the `CertificateBackend` trust anchor. Use `cargo run` with a
custom tool or `openssl` / `ssh-keygen` to generate an Ed25519 keypair:

```sh
# Using OpenSSL
openssl genpkey -algorithm ed25519 -out server.key
openssl pkey -in server.key -pubout -out server.pub
```

The raw 32-byte public key bytes extracted from the DER-encoded public key
form the `TrustAnchor::key` field for `CertificateBackend`.

### 4.3 Key Storage Recommendations

- Store secret key material in a secrets manager or an encrypted vault.
- Do not commit key files to the repository.
- Rotate static keys and token signing keys at least every 90 days.
- The Ed25519 private key for certificate signing must never be placed on the
  server; the server holds only the trust anchor public key.

---

## 5. Monitoring and Metrics

### 5.1 Metrics Counters

`telemetry::metrics::MetricsRegistry` maintains four saturating u64 counters.
The `snapshot()` method returns a `MetricsSnapshot` that can be serialized and
exported to a monitoring system.

| Counter              | Incremented by                                                  |
|----------------------|-----------------------------------------------------------------|
| `handshake_success`  | Each time a Noise handshake reaches `HandshakeState::Established` |
| `fallback_count`     | Each time `ModeNegotiator` triggers a transport fallback          |
| `loss_events`        | Each time `transport::loss` detects a packet loss event           |
| `auth_rejections`    | Each time `AuthCoordinator::authenticate` returns `AuthError::Rejected` |

Counters use `saturating_add` and will not overflow; they wrap to
`u64::MAX` rather than panicking.

### 5.2 Health Probe Format

`telemetry::log::emit_health_probe` produces a single-line status string
suitable for log aggregation and alerting:

```
health component=<name> status=<ok|degraded> detail=<text>
```

Example outputs:

```
health component=client_runtime status=ok detail=running
health component=crypto status=degraded detail=cipher_failure_count=3
```

Health probes should be emitted periodically (recommended: every 30 seconds)
by the runtime and ingested by a log aggregator such as Loki, Splunk, or
Elasticsearch.

---

## 6. Log Format

All structured log lines are emitted by `tracing-subscriber` to stderr.
Two line shapes are produced:

### 6.1 Event Lines

Emitted by `telemetry::log::format_event`:

```
event code=<event_code> detail=<key=value pairs>
```

| Event Code           | Meaning                                                          |
|----------------------|------------------------------------------------------------------|
| `startup`            | Daemon has parsed configuration and is entering the event loop.  |
| `runtime_ready`      | The runtime backend (epoll / kqueue / IOCP) has initialized.     |
| `handshake_success`  | A Noise handshake completed successfully with a peer.            |
| `fallback_triggered` | Transport fallback from UDP-TLS to TCP-TLS was initiated.        |
| `loss_observed`      | Packet loss was detected by `transport::loss`.                   |
| `auth_rejected`      | An inbound authentication attempt was rejected.                  |

Example:

```
event code=startup detail=mode=client server=203.0.113.1:443 transport=auto
event code=handshake_success detail=peer=203.0.113.1:443 epoch=0
event code=fallback_triggered detail=count=1
```

### 6.2 Health Probe Lines

```
health component=<name> status=<ok|degraded> detail=<text>
```

### 6.3 Error Lines

Runtime errors that cause process exit are written to stderr as:

```
event code=startup detail=state=error reason=<error message>
```

Config parse and validation errors appear as:

```
config parse error: missing required configuration key: client.server
config validation error: invalid configuration value for key: transport.fallback_timeout
```

---

## 7. Troubleshooting

### 7.1 Daemon Exits with Status 1

The exit-1 path is `format_event(EventCode::Startup, "state=error reason=...")`.
Check stderr for the `reason=` field. Common causes:

| Reason text                                              | Root cause                                               |
|----------------------------------------------------------|----------------------------------------------------------|
| `cannot read config /etc/apate/apate.conf: ...`          | Config file does not exist or is not readable.           |
| `config parse error: unsupported configuration key: X`   | An unrecognized key appears in the config file.          |
| `config parse error: invalid configuration value for key: transport.mode` | A value outside `auto/udp/tcp/quic_mask` was specified. |
| `config validation error: missing required configuration key: client.server` | `client.server` is missing in client mode.              |
| `config validation error: missing required configuration key: auth.methods` | `auth.methods` is empty in server mode.                  |
| `config validation error: invalid configuration value for key: transport.fallback_timeout` | `transport.fallback_timeout` is set to `0`.            |

### 7.2 Daemon Exits with Status 2

Status 2 is the CLI parse error path. The argument parser printed a message
such as `no command specified` or `unknown command: foo`. Run `apate help` for
usage.

### 7.3 Transport Fallback Triggered Repeatedly

If `event code=fallback_triggered` appears on every connection attempt, the
UDP path to the server is blocked by an intermediate firewall or NAT device.
Set `transport.mode = "tcp"` to skip the UDP attempt:

```
transport.mode = "tcp"
```

Alternatively, if TLS camouflage on port 443 is required, ensure
`stealth.profile` is set to a profile whose ALPN matches what the camouflage
layer presents.

### 7.4 Authentication Rejections (auth_rejected)

`event code=auth_rejected` means `AuthCoordinator::authenticate` returned
`AuthError::Rejected`. The error message is intentionally fixed at
"authentication rejected" with no additional detail to prevent information
leakage.

Verify:
- The client is presenting the correct auth method (one that is in `auth.methods`).
- For `static_key`: the key bytes match one entry in `StaticKeyBackend`'s key list.
- For `token`: the token signature matches the `signing_key` configured in `TokenBackend`, the `expires_at_unix` has not passed, and `audience` / `issuer` match if `TokenPolicy` checks them.
- For `certificate`: the `CertificateClaims.issuer` matches one of the `TrustAnchor.issuer` values in `CertificateBackend`, the `not_after_unix` has not passed, and the signature verifies against the anchor's `key`.

### 7.5 High Loss Events

`event code=loss_observed` indicates the transport layer is detecting dropped
packets. Check:

- Network path quality (run `mtr` or `ping` to the server endpoint).
- Whether FEC is helping: `FecController` automatically activates
  `SingleParity` at >= 5% loss and `DoubleParity` at >= 20% loss. If loss
  exceeds 20% consistently, investigate the underlying network path.
- Whether the transport kind is `tcp_tls`: `FecController` disables parity
  in TCP mode because TCP provides reliable delivery; loss events on TCP
  indicate TCP retransmissions, which are normal.

### 7.6 Stealth Profile Override Not Applied

If a custom profile specified in `stealth.profile_path` does not appear to
take effect:

1. Verify the file is readable by the process user.
2. Verify the file follows the `key = value` format and all required fields
   (`name`, `alpn`, `min_packet_size`, `max_packet_size`, `min_jitter_ms`,
   `max_jitter_ms`) are present.
3. Check stderr for `ProfileError::ProfileOverrideReadFailed` or
   `ProfileError::InvalidProfile { field: ... }` messages.

### 7.7 Verifying the Binary Version

```sh
apate version
```

Output:

```
apate 0.1.0
```

The version string is taken from `CARGO_PKG_VERSION` at compile time. If the
installed binary version does not match the expected release, rebuild from the
correct tag.

### 7.8 cargo deny Check Fails in CI

Run locally to reproduce:

```sh
cargo install cargo-deny
cargo deny check
```

Common failures:

| `cargo deny` section | Failure message                                    | Resolution                                              |
|----------------------|----------------------------------------------------|---------------------------------------------------------|
| `[advisories]`       | `crate X is affected by advisory RUSTSEC-YYYY-NNNN` | Update the affected crate or add an explicit ignore with justification in `deny.toml`. |
| `[licenses]`         | `crate X has license GPL-2.0`                      | Replace the crate with a permissively-licensed alternative. |
| `[bans]`             | `duplicate crate X versions`                       | Unify the version by adding a `[patch]` override in `Cargo.toml`. |
