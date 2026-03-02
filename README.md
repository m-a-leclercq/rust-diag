# rust-diags

A fast, single-binary Elasticsearch diagnostics collector. It queries a configurable set of API endpoints against a live cluster, then packages the results into a `.zip` archive ready for support or analysis.

## Installation

### Pre-built binaries

Download the latest release for your platform from the [Releases](../../releases) page:

| Platform | Binary |
|---|---|
| Linux x86_64 | `rust-diags-linux-x86_64` |
| macOS Intel | `rust-diags-macos-x86_64` |
| macOS Apple Silicon | `rust-diags-macos-aarch64` |
| Windows x86_64 | `rust-diags-windows-x86_64.exe` |

### Build from source

Requires Rust 1.85+ (edition 2024).

```bash
cargo build --release
# binary: target/release/rust-diags
```

## Usage

```
rust-diags --url <URL> [OPTIONS]
```

### Options

| Flag | Description |
|---|---|
| `--url <URL>` | Elasticsearch base URL (**required**) |
| `--user <USER>` | Username for Basic auth |
| `--password <PASS>` | Password for Basic auth |
| `--api-key <base64>` | API key in `encoded` (base64) format |
| `--cacert <PATH>` | Path to a CA certificate PEM file |
| `--insecure` | Disable TLS certificate verification |
| `--output <DIR>` | Directory to write the archive (default: current directory) |

### Examples

**Local cluster, no auth:**
```bash
rust-diags --url http://localhost:9200
```

**Username and password:**
```bash
rust-diags --url https://my-cluster:9200 \
  --user elastic --password changeme
```

**API key:**
```bash
rust-diags --url https://my-cluster:9200 \
  --api-key VnVhQ2ZHY0JDZGJrUW0tZTVhT3g6dWkybHAyYXhUTm1zeWFrdzl0dk5udw==
```

**Elastic Cloud / HTTPS with custom CA:**
```bash
rust-diags --url https://my-cluster:9200 \
  --user elastic --password changeme \
  --cacert /path/to/ca.crt \
  --output /tmp
```

**Skip TLS verification (dev/test only):**
```bash
rust-diags --url https://localhost:9200 --insecure
```

## Output

The tool prints the path to the resulting archive on stdout:

```
/tmp/docker-cluster-diagnostics-20260302-153801.zip
```

The archive contains one file per API endpoint, organised into subdirectories:

```
docker-cluster-diagnostics-20260302-153801.zip
├── cat/
│   ├── cat_health.txt
│   ├── cat_nodes.txt
│   └── ...
├── commercial/
│   ├── license.json
│   └── ...
├── cluster_health.json
├── cluster_state.json
└── ...
```

### Exit behaviour

| Condition | Behaviour |
|---|---|
| Connection / auth failure | Fatal — exits with error message |
| Individual endpoint non-2xx (`showErrors: true`) | Logged as `WARN`, collection continues |
| Individual endpoint non-2xx (`showErrors: false`) | Logged as `INFO` — expected when a feature is unlicensed or the cluster is not in a state where the API can return data (e.g. `allocation/explain` with no unassigned shards) |

## Logging

Log level is controlled via the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug rust-diags --url http://localhost:9200
```

## Licensing

The Rust source code in this repository is licensed under the **Apache License 2.0** — see [`LICENSE`](LICENSE).

`resources/apis.yaml` is derived from [elastic/support-diagnostics](https://github.com/elastic/support-diagnostics) and is licensed under the **Elastic License 2.0 (ELv2)**. See [`resources/NOTICE`](resources/NOTICE).
