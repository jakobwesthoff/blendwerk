# CLI and Server Behavior

Invoking the `blendwerk` binary: arguments, protocol modes, TLS, and runtime
behavior.

## Invocation

```bash
blendwerk <DIRECTORY> [OPTIONS]
```

`<DIRECTORY>` is the mock root and must exist, otherwise startup fails. On
startup blendwerk scans it, prints every discovered route
(e.g. `Get /api/users/:id`), and starts serving. A scan error (such as broken
frontmatter in any file) aborts startup.

| Option | Default | Meaning |
|--------|---------|---------|
| `-p, --http-port <PORT>` | 8080 | HTTP port |
| `-s, --https-port <PORT>` | 8443 | HTTPS port |
| `--http-only` | off | Serve HTTP only (conflicts with `--https-only`) |
| `--https-only` | off | Serve HTTPS only (conflicts with `--http-only`) |
| `--cert-mode <MODE>` | `self-signed` | `none`, `self-signed`, or `custom` |
| `--cert-file <FILE>` | — | Certificate file; required when `--cert-mode custom` |
| `--key-file <FILE>` | — | Private key file; required when `--cert-mode custom` |
| `--request-log <DIR>` | off | Log every request into this directory |
| `--request-log-format <FMT>` | `json` | `json` or `yaml` |

Both servers bind `0.0.0.0`, so mocks are reachable from other machines and
from containers.

## Protocol Modes

By default blendwerk serves **both** HTTP (:8080) and HTTPS (:8443) with a
self-signed certificate generated at startup.

```bash
blendwerk ./mocks                    # HTTP :8080 + HTTPS :8443 (self-signed)
blendwerk ./mocks --http-only        # HTTP only; --cert-mode none is equivalent
blendwerk ./mocks --https-only       # HTTPS only
blendwerk ./mocks --cert-mode custom --cert-file server.crt --key-file server.key
```

Combining `--https-only` with `--cert-mode none` disables both servers and
startup fails.

The self-signed certificate is not trusted by clients; use `curl -k` or the
equivalent insecure-TLS flag when testing against the HTTPS port.

## Testing a Mock

```bash
curl http://localhost:8080/api/users
curl -k https://localhost:8443/api/users
curl -i -X POST http://localhost:8080/api/users -d '{"name":"Alice"}'
```

Use `-i` to see the status and headers defined in the mock's frontmatter.

## Runtime Behavior

- **Hot reload:** the mock directory is watched recursively; changes apply
  after a ~100 ms debounce without restart. A failed reload keeps the old
  routes and logs the error.
- **Shutdown:** SIGINT (Ctrl+C) and SIGTERM trigger graceful shutdown.
- **Containers:** when running as PID 1, blendwerk automatically reaps
  zombies and handles signals; no init wrapper or configuration is needed:

  ```dockerfile
  FROM scratch
  COPY blendwerk /blendwerk
  COPY mocks /mocks
  ENTRYPOINT ["/blendwerk"]
  CMD ["/mocks"]
  ```

## Installation

```bash
cargo install blendwerk
```
