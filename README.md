# Apate

A stealth VPN protocol that tunnels IP traffic through encrypted connections disguised as legitimate TLS and QUIC traffic. Designed to operate in network environments where VPN protocols are detected and blocked by deep packet inspection (DPI).

## How It Works

Apate wraps VPN tunnel traffic inside protocol frames that are indistinguishable from real browser HTTPS/QUIC sessions. A Noise protocol handshake (X25519 + Ed25519) establishes an encrypted channel, then IP packets flow through the selected transport:

```
Application  -->  TUN Device  -->  Apate Protocol  -->  UDP/TCP/QUIC  -->  Internet
                                       |
                              Noise Encryption Layer
                              TLS/QUIC Camouflage
                              Browser Fingerprint
```

When a DPI system probes the server, a facade responder returns plausible HTTP responses mimicking a web server, deflecting active probing attacks.

## Features

- **Transport Modes**: UDP-over-TLS, TCP-over-TLS, and QUIC (RFC 9000 compliant via quinn-proto)
- **Automatic Fallback**: Configurable transport negotiation with fallback chain (UDP -> TCP -> QUIC)
- **Noise Protocol**: X25519 Diffie-Hellman key exchange, Ed25519 authentication, ChaCha20-Poly1305 / AES-256-GCM dual AEAD
- **Browser Fingerprinting**: TLS ClientHello profiles matching Chrome 131, Firefox 130, Safari 18
- **Probe Deflection**: Facade responder serves realistic HTTP responses to DPI probes
- **Multi-Platform**: Linux (epoll, io_uring), macOS (kqueue, utun), Windows (IOCP, WinTUN), FreeBSD (kqueue)
- **Multi-Auth**: Static key, token, and certificate authentication backends
- **Split Tunneling**: Configurable routing with per-prefix tunnel/bypass decisions
- **DNS Protection**: DoH forwarding, plain DNS, and fallback modes with leak prevention
- **Forward Error Correction**: Adaptive single/double parity FEC for lossy links
- **Connection Migration**: Cryptographic migration proofs for endpoint changes
- **Post-Quantum Ready**: ML-KEM-768 key encapsulation available alongside X25519

## Building

Requires Rust stable toolchain (1.85+, 2024 edition).

```bash
cargo +stable build --release
```

Cross-platform release binaries:

```bash
cargo +stable build --release --target x86_64-unknown-linux-gnu
cargo +stable build --release --target aarch64-apple-darwin
cargo +stable build --release --target x86_64-pc-windows-msvc
```

## Usage

```bash
# Generate a keypair
apate gen-key

# Start client
apate client -c /etc/apate/apate.conf

# Start server
apate server -c /etc/apate/apate.conf

# Print version
apate version
```

## Configuration

Apate uses a key-value configuration file:

```
client.server = "203.0.113.10:443"
auth.methods = ["static_key"]
transport.mode = "auto"
transport.fallback_timeout_secs = 3
routing.mode = "full"
dns.mode = "doh"
stealth.profile = "chrome_131"
stealth.facade_on_auth_failure = true
server.listen = "0.0.0.0:443"
```

### Transport Modes

| Mode         | Config Value   | Description                                      |
|--------------|----------------|--------------------------------------------------|
| Automatic    | `auto`         | Try UDP first, fall back to TCP, then QUIC        |
| UDP only     | `udp`          | Direct UDP-over-TLS                               |
| TCP only     | `tcp`          | TCP-over-TLS (traverses restrictive firewalls)    |
| QUIC         | `quic_mask`    | Full QUIC protocol (RFC 9000, AEAD, header protection) |

### Stealth Profiles

| Profile        | Config Value     |
|----------------|------------------|
| Chrome 131     | `chrome_131`     |
| Firefox 130    | `firefox_130`    |
| Safari 18      | `safari_18`      |

## Architecture

```
src/
  transport/    Network I/O, framing, congestion control, FEC, migration
  tunnel/       Platform TUN adapters (Linux, macOS, Windows, FreeBSD)
  runtime/      Synchronous poll-based event loop (kqueue/epoll/io_uring/IOCP)
  noise/        Noise protocol handshake and symmetric state
  crypto/       AEAD, KDF, key exchange, signatures, RNG
  stealth/      TLS camouflage, browser fingerprints, facade responder
  auth/         Authentication coordinator and backends
  routing/      Split/full tunnel routing, DNS forwarding
  config/       Configuration parser and browser profiles
  cli/          Command dispatch and entry points
  telemetry/    Structured event logging
```

Detailed documentation:

- [Architecture](docs/architecture.md) -- module decomposition, data flow, state machines
- [Wire Format](docs/wire-format.md) -- binary frame specification
- [Operations](docs/operations.md) -- deployment and operational guidance

## Testing

```bash
# All tests
cargo +stable test

# Single test
cargo +stable test auto_mode_falls_back

# Benchmarks
cargo +stable bench --bench packet_path -- --sample-size 10
```

TUN adapter tests require root/admin privileges. They skip gracefully in unprivileged environments.

## Platform Support

| Platform | I/O Backend      | TUN Adapter               | Socket API |
|----------|------------------|---------------------------|------------|
| Linux    | epoll, io_uring  | /dev/net/tun (ioctl)      | libc       |
| macOS    | kqueue           | utun (PF_SYSTEM)          | libc       |
| Windows  | IOCP             | WinTUN (runtime DLL load) | Winsock    |
| FreeBSD  | kqueue           | /dev/tunN                 | libc       |

Windows TUN support requires [WinTUN](https://www.wintun.net/) DLL installed on the system.

## Security

- Noise NK handshake pattern with X25519 ephemeral keys
- Ed25519 signature-based authentication proofs
- BLAKE3-based key derivation
- Constant-time operations via the `subtle` crate
- Sensitive key material zeroed on drop via `zeroize`
- No plaintext credentials in configuration (key material is hex-encoded)

## License

MIT OR Apache-2.0
