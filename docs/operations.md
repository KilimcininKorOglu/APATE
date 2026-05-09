# Apate Operations Guide

This document covers installation, configuration, execution, key management,
monitoring, log interpretation, and troubleshooting for the Apate stealth VPN
protocol daemon.

---

## 1. Installation

### 1.1 Building from Source

Prerequisites:

- Rust toolchain 1.85 or later (`rustup toolchain install stable`)
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

| Key                                | Required | Default          | Valid Values                                                      | Description                                                                                            |
|------------------------------------|----------|------------------|-------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------|
| `client.server`                    | Client   | (none)           | `<host>:<port>` string                                            | Server endpoint the client connects to. Required in client mode; validated to be non-empty.             |
| `server.listen`                    | Server   | `0.0.0.0:443`    | `<bind_addr>:<port>` string                                       | Address and port the server listens on. Parsed as `SocketAddr` at startup.                             |
| `transport.mode`                   | No       | `auto`           | `auto`, `udp`, `tcp`, `quic_mask`                                 | Transport selection strategy. `auto` attempts UDP-TLS first, falls back to TCP-TLS on timeout.         |
| `transport.fallback_timeout`       | No       | `3`              | Integer >= 1                                                      | Seconds to wait before the `auto` mode triggers a transport fallback. `0` is rejected as invalid.      |
| `stealth.profile`                  | No       | `chrome_131`     | `chrome_131`, `firefox_130`, `safari_18`                          | Built-in stealth profile controlling ALPN, packet sizes, jitter, and traffic shaping.                  |
| `stealth.profile_path`             | No       | (empty)          | Filesystem path string                                            | Path to a custom profile file that overrides the built-in profile. Takes priority when set.             |
| `stealth.facade_on_auth_failure`   | No       | `true`           | `true`, `false`                                                   | When `true`, serves fake HTTP responses to failed auth attempts instead of closing the connection.     |
| `auth.methods`                     | Server   | (none)           | `["static_key"]`, `["token"]`, `["certificate"]`, or combinations | Ordered list of authentication backends. Required in server mode.                                      |
| `crypto.post_quantum`              | No       | `true`           | `true`, `false`                                                   | When `true`, the handshake uses hybrid X25519 + ML-KEM key exchange. When `false`, X25519 only.        |
| `crypto.rekey_interval_secs`       | No       | `60`             | u32 >= 1                                                          | Time-based rekey trigger: initiate REKEY after this many seconds since last key rotation.               |
| `crypto.rekey_interval_bytes`      | No       | `1073741824`     | u64 >= 1                                                          | Byte-based rekey trigger: initiate REKEY after this many bytes transmitted under the current key.      |
| `routing.mode`                     | No       | `full`           | `full`, `split`                                                   | `full` sends all traffic through the tunnel. `split` routes only configured CIDRs through the tunnel.  |
| `dns.mode`                         | No       | `doh`            | `doh`, `plain`, `fallback`                                        | DNS resolution strategy. `doh` sends DNS over HTTPS through the tunnel. `plain` uses system resolver.  |

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

See Section 8.5 (Server) and Section 9.6 (Client) for complete systemd
and launchd service configurations.

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

---

## 8. Server Deployment Guide

This section walks through deploying Apate on a real VPS or dedicated server.

### 8.1 Prerequisites

- A Linux VPS (Ubuntu 22.04+ / Debian 12+ recommended) with a public IP
- Port 443 open in the hosting provider's firewall
- Root or sudo access
- Rust toolchain installed (or pre-compiled binary)

### 8.2 Build on the Server

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
git clone https://github.com/KilimcininKorOglu/APATE.git
cd APATE
cargo +stable build --release
```

Or build locally and copy the binary:

```sh
# On your local machine (cross-compile for Linux)
cargo +stable build --release --target x86_64-unknown-linux-gnu

# Copy to server
scp target/x86_64-unknown-linux-gnu/release/apate root@your-server:/usr/local/bin/
```

### 8.3 Install and Configure

```sh
# Install binary
install -m 0755 target/release/apate /usr/local/bin/apate

# Grant TUN capability (avoids running as root)
setcap cap_net_admin=ep /usr/local/bin/apate

# Create config directory
mkdir -p /etc/apate

# Generate a keypair
apate gen-key
# Output: public_key=<64-hex-chars>
# Save the public key — you will need it on the client side
```

Create the server config file:

```sh
cat > /etc/apate/apate.conf << 'EOF'
server.listen = "0.0.0.0:443"
transport.mode = "quic_mask"
auth.methods = ["static_key"]
stealth.profile = "chrome_131"
stealth.facade_on_auth_failure = true
crypto.post_quantum = true
crypto.rekey_interval_secs = 60
EOF
```

An example config is also available at `examples/server.conf`.

### 8.4 Firewall Configuration

```sh
# UFW (Ubuntu/Debian)
ufw allow 443/udp    # QUIC mode
ufw allow 443/tcp    # TCP fallback / probe deflection
ufw enable

# Or iptables
iptables -A INPUT -p udp --dport 443 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT
```

For QUIC mode, UDP port 443 must be open. For TCP fallback and probe
deflection, TCP port 443 must also be open.

### 8.5 Create Systemd Service

```sh
cat > /etc/systemd/system/apate.service << 'EOF'
[Unit]
Description=Apate Stealth VPN Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/apate server --config /etc/apate/apate.conf
Restart=on-failure
RestartSec=5s
AmbientCapabilities=CAP_NET_ADMIN
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/dev/net/tun

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now apate
```

### 8.6 Verify Server is Running

```sh
# Check service status
systemctl status apate

# Check logs
journalctl -u apate -f

# Expected startup log:
# event code=startup detail=mode=server-quic listen=0.0.0.0:443 backend=epoll
```

### 8.7 Transport Mode Selection

| Scenario                                      | Recommended Mode | Config Value   |
|-----------------------------------------------|------------------|----------------|
| Standard deployment (best stealth)            | QUIC             | `quic_mask`    |
| UDP blocked by ISP/firewall                   | TCP only         | `tcp`          |
| Unknown network conditions                    | Automatic        | `auto`         |
| Maximum compatibility                         | TCP only         | `tcp`          |

QUIC mode (`quic_mask`) provides the strongest DPI evasion because the
traffic uses real QUIC protocol framing (RFC 9000) with AEAD encryption
and header protection. The server automatically generates self-signed
certificates and rotates them every 1-2 hours.

---

## 9. Client Setup Guide

### 9.1 Install Client Binary

Build or download the binary for your platform:

```sh
# macOS (Apple Silicon)
cargo +stable build --release --target aarch64-apple-darwin

# Linux
cargo +stable build --release --target x86_64-unknown-linux-gnu

# Windows
cargo +stable build --release --target x86_64-pc-windows-msvc
```

Install:

```sh
# Linux/macOS
install -m 0755 target/release/apate /usr/local/bin/apate
setcap cap_net_admin=ep /usr/local/bin/apate   # Linux only

# Windows: copy apate.exe to a PATH directory
# Windows: install WinTUN driver from https://www.wintun.net/
```

### 9.2 Create Client Config

```sh
mkdir -p /etc/apate

cat > /etc/apate/apate.conf << 'EOF'
client.server = "203.0.113.10:443"
transport.mode = "auto"
auth.methods = ["static_key"]
stealth.profile = "chrome_131"
routing.mode = "full"
dns.mode = "doh"
crypto.post_quantum = true
EOF
```

Replace `203.0.113.10` with your server's public IP address.
An example config is also available at `examples/client.conf`.

### 9.3 Transport Mode Matching

The client transport mode must be compatible with the server:

| Server Mode  | Compatible Client Modes      |
|--------------|------------------------------|
| `quic_mask`  | `quic_mask`, `auto`          |
| `auto`       | `auto`, `udp`, `tcp`         |
| `udp`        | `udp`, `auto`                |
| `tcp`        | `tcp`, `auto`                |

When the server runs in `quic_mask` mode and the client uses `auto`, the
client will attempt UDP-TLS and TCP-TLS first (which will fail), then
fall back to QUIC.

### 9.4 Connect to Server

```sh
# With default config path
sudo apate client

# With custom config path
sudo apate client --config /path/to/client.conf

# With verbose logging
sudo apate client --config /path/to/client.conf --verbose
```

Root/sudo is required for TUN device access. Expected output:

```
event code=startup detail=mode=client server=203.0.113.10:443 transport=auto backend=kqueue
event code=handshake_success detail=transport=UdpTls endpoint=203.0.113.10:443
event code=runtime_ready detail=tunnel=utun7 mtu=1400
```

### 9.5 Verify the Tunnel

```sh
# Check TUN interface exists
ip addr show apate0          # Linux
ifconfig utun7               # macOS

# Test connectivity through the tunnel
ping -c 3 8.8.8.8

# Verify your public IP has changed
curl https://ifconfig.me
```

### 9.6 Running as a System Service (Client)

**Linux:**

```sh
cat > /etc/systemd/system/apate-client.service << 'EOF'
[Unit]
Description=Apate Stealth VPN Client
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/apate client --config /etc/apate/apate.conf
Restart=on-failure
RestartSec=10s
AmbientCapabilities=CAP_NET_ADMIN
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now apate-client
```

**macOS (launchd):**

```sh
cat > /Library/LaunchDaemons/com.apate.client.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.apate.client</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/apate</string>
        <string>client</string>
        <string>--config</string>
        <string>/etc/apate/apate.conf</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
EOF

launchctl load /Library/LaunchDaemons/com.apate.client.plist
```

---

## 10. Key Exchange Workflow

Apate uses pre-shared static keys for authentication. The key exchange
must happen out-of-band (not through the VPN itself).

### 10.1 Generate Keys

On both the server and the client:

```sh
apate gen-key
# Output: public_key=3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29
```

The `gen-key` command generates a random 32-byte secret key using OS
entropy, derives the X25519 public key, and prints only the public key.
The secret key is not persisted or printed.

### 10.2 Exchange Public Keys

1. Generate a keypair on the **server**: `apate gen-key` -> note the public key
2. Generate a keypair on the **client**: `apate gen-key` -> note the public key
3. Share public keys through a secure channel (SSH, Signal, in person)
4. Configure each side with the peer's public key

### 10.3 Security Considerations

- Never transmit private keys over the network
- Rotate static keys at least every 90 days
- Use `certificate` auth method for production deployments where key
  rotation needs to be automated
- The `token` auth method supports time-limited access with expiration

---

## 11. DPI Evasion Features

Apate includes several layers of DPI evasion that operate automatically
once configured via the stealth profile.

### 11.1 Automatic Features (No Configuration Needed)

| Feature                  | Description                                                         |
|--------------------------|---------------------------------------------------------------------|
| Traffic Shaping          | Markov chain models browser traffic patterns (packet sizes, timing) |
| Chaff Traffic            | Random packets during idle periods prevent silence-burst detection   |
| Decoy Streams            | Fake HTTP/3 streams multiplexed alongside real VPN data (QUIC mode) |
| Session Rotation         | Connection teardown/rebuild every 15-45 minutes with fresh IDs      |
| Certificate Rotation     | Server generates new TLS certificates every 1-2 hours (QUIC mode)  |
| Asymmetric Padding       | Download traffic padded more than upload to match browser ratios    |
| Probe Deflection         | Fake HTTP responses served to unauthenticated connections           |

### 11.2 Configurable Options

| Config Key                         | Effect                                                  |
|------------------------------------|---------------------------------------------------------|
| `stealth.profile`                  | Controls TLS fingerprint and traffic shaping profile    |
| `stealth.facade_on_auth_failure`   | Enables/disables probe deflection HTTP responses        |
| `stealth.profile_path`             | Custom stealth profile for fine-tuned packet parameters  |

### 11.3 Profile Recommendations

| Use Case                                  | Profile        | Transport    |
|--------------------------------------------|---------------|-------------|
| Countries with advanced DPI (China, Iran)  | `chrome_131`  | `quic_mask` |
| Corporate networks with SSL inspection    | `firefox_130` | `tcp`       |
| General purpose                           | `chrome_131`  | `auto`      |
