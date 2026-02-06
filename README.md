# blendwerk

A file-based mock HTTP/HTTPS server. Define API responses as files in a directory structure. Changes are automatically reloaded. No databases, no config files, just directories and text files.

> **blendwerk** [ˈblɛntvɛrk] — German: illusion, deceptive appearance.

## Installation

```bash
cargo install blendwerk
```

## Quick Start

```bash
# Create a mock
mkdir -p mocks/api/users
cat > mocks/api/users/GET.json << 'EOF'
---
status: 200
---
{"users": [{"id": 1, "name": "Alice"}]}
EOF

# Run
blendwerk ./mocks

# Test
curl http://localhost:8080/api/users
```

<!-- docs:start -->
## Documentation

Point blendwerk at a directory and it serves mock HTTP/HTTPS responses based on the file structure. No configuration needed — just files and folders.

```bash
blendwerk <DIRECTORY> [OPTIONS]
```

## Core Concepts

### Directory Structure Maps to Routes

Your directory structure IS your API. Folders become URL paths, filenames become HTTP methods. That's it.

```bash
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

**Route Matching:** Routes use first-match-wins ordering. Both static routes and `[param]` routes are matched in discovery order.

**Error Responses:**
- `404 Not Found` — No route matches the path
- `405 Method Not Allowed` — Path exists but method isn't defined

**Query Parameters:** Query strings don't affect route matching — all requests to a path use the same mock regardless of query parameters. However, query parameters are captured in request logs.

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

```bash
Usage: blendwerk [OPTIONS] <DIRECTORY>

Arguments:
  <DIRECTORY>
          Directory containing mock responses

Options:
  -p, --http-port <HTTP_PORT>
          HTTP port
          [default: 8080]

  -s, --https-port <HTTPS_PORT>
          HTTPS port
          [default: 8443]

      --http-only
          Only serve HTTP (no HTTPS)

      --https-only
          Only serve HTTPS (no HTTP)

      --cert-mode <CERT_MODE>
          Certificate mode

          Possible values:
          - none:        No HTTPS, HTTP only
          - self-signed: Generate self-signed certificate on startup
          - custom:      Use custom certificate files

          [default: self-signed]

      --cert-file <CERT_FILE>
          Path to certificate file (required for custom cert mode)

      --key-file <KEY_FILE>
          Path to private key file (required for custom cert mode)

      --request-log <REQUEST_LOG>
          Directory to log all incoming requests

      --request-log-format <REQUEST_LOG_FORMAT>
          Format for request logs

          [default: json]
          [possible values: json, yaml]

  -h, --help
          Print help

  -V, --version
          Print version
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

### Request Logging

blendwerk can log all incoming requests to a directory structure that mirrors your API routes. This is useful for debugging, testing, and understanding how your mock API is being used.

**Enable request logging:**

```bash
blendwerk ./mocks --request-log ./request-logs
```

**Directory structure:**

```bash
request-logs/
├── api/
│   └── users/
│       ├── GET/
│       │   ├── 2025-01-28T15-30-45.123456Z_01HQKP6J9Z0000000000000000.json
│       │   └── 2025-01-28T15-31-12.456789Z_01HQKP7A1A0000000000000000.json
│       └── POST/
│           └── 2025-01-28T15-32-00.789012Z_01HQKP8B2B0000000000000000.json
```

**Log file format:**

Each request is logged as a separate file containing complete request and response information:

```json
{
  "metadata": {
    "timestamp": "2025-01-28T15-30-45.123456Z",
    "request_id": "01HQKP6J9Z0000000000000000"
  },
  "request": {
    "method": "GET",
    "uri": "/api/users?page=2",
    "path": "/api/users",
    "query": "page=2",
    "headers": {
      "user-agent": "curl/8.0.0",
      "accept": "*/*"
    },
    "body": null,
    "matched_route": "/api/users"
  },
  "response": {
    "status": 200,
    "headers": {
      "content-type": "application/json"
    },
    "body": "{\"users\": [...]}",
    "delay_ms": 0
  }
}
```

**YAML format:**

```bash
blendwerk ./mocks --request-log ./request-logs --request-log-format yaml
```

Filenames use ISO 8601 timestamps plus ULIDs for sortability and uniqueness. Logging happens asynchronously and doesn't block responses. 404s are logged to their requested paths (e.g., a request to `/api/nonexistent` creates a log file in `request-logs/api/nonexistent/GET/`).

## Route Matching

When multiple routes could match a request, blendwerk uses **first-match-wins** ordering. Routes are matched in the order they're discovered during directory scanning.

### Static vs Dynamic Routes

Static routes (exact paths) and dynamic routes (with `[param]` segments) are treated equally — the first match wins. If you need a specific path to take precedence:

```bash
mocks/api/users/
├── admin/
│   └── GET.json      # GET /api/users/admin (static)
└── [id]/
    └── GET.json      # GET /api/users/:id (dynamic)
```

Both routes exist, and requests to `/api/users/admin` will match the static route if it's discovered first.

### Multiple Path Parameters

You can use multiple `[param]` segments for nested resources:

```bash
mocks/api/users/[userId]/posts/[postId]/
├── GET.json          # GET /api/users/:userId/posts/:postId
├── PUT.json          # PUT /api/users/:userId/posts/:postId
└── DELETE.json       # DELETE /api/users/:userId/posts/:postId
```

## Query Parameters

Query strings do **not** affect route matching — all requests to a path use the same mock response regardless of query parameters:

```bash
# All these hit the same mock: mocks/api/users/GET.json
curl http://localhost:8080/api/users
curl http://localhost:8080/api/users?page=1
curl http://localhost:8080/api/users?page=2&limit=10
```

However, query parameters **are logged** when request logging is enabled, so you can see exactly what your application is requesting.

## Request Logging Details

Understanding the logged fields:

| Field | Description |
|-------|-------------|
| `path` | The literal request path (e.g., `/api/users/42`) |
| `matched_route` | The route pattern that matched (e.g., `/api/users/:id`) |
| `query` | Query string if present, otherwise `null` |

**404 requests** are also logged to their requested paths. A request to `/nonexistent/path` creates a log file at `request-logs/nonexistent/path/GET/...`

## Cookbook

**RESTful CRUD API:**

```bash
mocks/api/users/
├── GET.json                  # List users
├── POST.json                 # Create user (status: 201)
└── [id]/
    ├── GET.json              # Get single user
    ├── PUT.json              # Update user
    └── DELETE.json           # Delete user (status: 204, empty body)
```

**Error responses with custom headers:**

```yaml
# mocks/api/admin/GET.json
---
status: 403
headers:
  X-Error-Code: FORBIDDEN
  X-Error-Message: Admin access required
---
{"error": "forbidden", "message": "Admin access required"}
```

**CORS preflight response:**

```yaml
# mocks/api/data/OPTIONS.json
---
status: 204
headers:
  Access-Control-Allow-Origin: "*"
  Access-Control-Allow-Methods: GET, POST, PUT, DELETE
  Access-Control-Allow-Headers: Content-Type, Authorization
---
```

**Simulating slow API (rate limiting test):**

```yaml
# mocks/api/heavy-operation/POST.json
---
delay: 3000
status: 202
---
{"status": "processing", "estimatedTime": "3 seconds"}
```

**Override Content-Type:**

```yaml
# mocks/api/legacy/GET.json - serve JSON with custom content type
---
headers:
  Content-Type: application/vnd.api+json
---
{"data": {"type": "users", "id": "1"}}
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

**Text Files Only:** Response files are read as UTF-8 text. Binary responses (images, PDFs) are not supported.

**Static Responses:** Responses are static — you cannot vary the response based on request body, headers, or query parameters. Each (method, path) combination always returns the same response.

<!-- docs:end -->

## Development

```bash
# Clone the repository
git clone https://github.com/jakobwesthoff/blendwerk.git
cd blendwerk

# Build
cargo build --release

# Run tests
cargo test

# Run from source
cargo run -- ./mocks
```

## License

This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.

Copyright (c) 2025 Jakob Westhoff <jakob@westhoffswelt.de>
