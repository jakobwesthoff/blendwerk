# blendwerk

A file-based mock HTTP/HTTPS server. Define API responses as files in a directory structure. Changes are automatically reloaded. No databases, no config files, just directories and text files.

> **blendwerk** [ˈblɛntvɛrk] — German: illusion, deceptive appearance.

## Quick Start

```bash
# Build
cargo build --release

# Create a mock
mkdir -p mocks/api/users
cat > mocks/api/users/GET.json << 'EOF'
---
status: 200
---
{"users": [{"id": 1, "name": "Alice"}]}
EOF

# Run
./target/release/blendwerk ./mocks

# Test
curl http://localhost:8080/api/users
```

## Core Concepts

### Directory Structure Maps to Routes

Your directory structure IS your API. Folders become URL paths, filenames become HTTP methods. That's it.

```
mocks/
├── api/
│   ├── users/
│   │   ├── GET.json          # GET /api/users
│   │   ├── POST.json         # POST /api/users
│   │   └── [id]/             # Path parameter
│   │       ├── GET.json      # GET /api/users/:id
│   │       ├── PUT.json      # PUT /api/users/:id
│   │       └── DELETE.json   # DELETE /api/users/:id
│   └── health/
│       └── GET.json          # GET /api/health
└── GET.html                  # GET /
```

**Rules:**
- Method names are case-insensitive (`GET.json`, `get.json`, `Get.json` all work)
- Use `[paramName]` directories for path parameters (matches any path segment)
- Hot-reload: changes to files are detected automatically

## Response Files

### Format

Response files use optional YAML frontmatter followed by the response body:

```yaml
---
status: 201
headers:
  X-Request-Id: abc-123
  Cache-Control: no-cache
delay: 100
---
{"created": true, "id": 42}
```

### Frontmatter Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `status` | integer | 200 | HTTP status code |
| `headers` | map | {} | Response headers |
| `delay` | integer | 0 | Delay in milliseconds before responding |

All fields are optional. Files without frontmatter return status 200.

### Content-Type

Automatically inferred from file extension (can of course be overridden in `headers`):

- `.json` → `application/json`
- `.html` → `text/html`
- `.xml` → `application/xml`
- `.txt` → `text/plain`

### Examples

**Error response:**
```yaml
# mocks/api/protected/GET.json
---
status: 401
headers:
  WWW-Authenticate: Bearer realm="api"
---
{"error": "unauthorized"}
```

**Simulating latency:**
```yaml
# mocks/api/slow/GET.json
---
delay: 2000
---
{"message": "This took 2 seconds"}
```

**Multiple methods:**
```bash
# mocks/api/items/GET.json
{"items": []}

# mocks/api/items/POST.json
---
status: 201
---
{"created": true}
```

## Configuration

### Command Line Options

```
blendwerk [OPTIONS] <DIRECTORY>

Arguments:
  <DIRECTORY>               Directory containing mock responses

Options:
  -p, --http-port <PORT>    HTTP port [default: 8080]
  -s, --https-port <PORT>   HTTPS port [default: 8443]
      --http-only           Only serve HTTP
      --https-only          Only serve HTTPS
      --cert-mode <MODE>    Certificate mode: none, self-signed, custom [default: self-signed]
      --cert-file <FILE>    Path to certificate file (for custom mode)
      --key-file <FILE>     Path to private key file (for custom mode)
  -h, --help                Print help
```

### HTTP/HTTPS Modes

**Default (HTTP + HTTPS with self-signed cert):**
```bash
blendwerk ./mocks
# HTTP on :8080, HTTPS on :8443
```

**HTTP only:**
```bash
blendwerk ./mocks --http-only
# or
blendwerk ./mocks --cert-mode none
```

**HTTPS only:**
```bash
blendwerk ./mocks --https-only
```

**Custom certificate:**
```bash
blendwerk ./mocks --cert-mode custom --cert-file server.crt --key-file server.key
```

## Docker Container Support

blendwerk properly handles running as PID 1, so you can run it directly in containers without worrying about zombie processes or signal handling. When running as PID 1 (the init process), it automatically:

- Reaps zombie processes
- Handles SIGTERM and SIGINT for graceful shutdown
- Forwards signals to child processes

This behavior is **autodetected** and requires no configuration:

```dockerfile
FROM scratch
COPY blendwerk /blendwerk
COPY mocks /mocks
ENTRYPOINT ["/blendwerk"]
CMD ["/mocks"]
```

## Limitations

**Memory Usage:** blendwerk loads all mock response files into memory at startup (and on hot-reload). This keeps things blazing fast for development and testing, but means you probably shouldn't throw gigabyte-sized video files or massive datasets at it. If you're mocking endpoints that return large binary chunks, keep an eye on your RAM.

**Production Use:** Look, I think blendwerk is pretty cool, and it's great for local development, integration testing, and temporary mock services. But it's not nginx. It's not built to be a battle-hardened production web server handling millions of requests. If you find yourself thinking "maybe I should use this in production for real traffic"... maybe take a step back and consider if you're solving the right problem. That said, for what it's designed to do - providing quick, file-based API mocks - it does it well.

## License

This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
