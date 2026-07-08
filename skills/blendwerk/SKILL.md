---
name: blendwerk
description: >-
  Work with blendwerk, the file-based mock HTTP/HTTPS server: run its CLI,
  create or modify mock directory trees (METHOD.json response files, [param]
  directories, YAML frontmatter for status/headers/delay), and read or analyze
  the request logs it writes. Use whenever the user mentions blendwerk, wants
  to mock or fake an HTTP/REST API with files and directories, needs to
  inspect, understand, or replay requests captured by a mock server, asks why
  a mock returns the wrong status or 404, or works inside a mocks/ or
  request-logs/ directory tree served by blendwerk.
---

# blendwerk

blendwerk serves mock HTTP/HTTPS responses straight from a directory tree:
folders are URL path segments, file names are HTTP methods, file contents are
the response. No config files; changes hot-reload automatically.

## Core Model

```
mocks/
├── api/
│   └── users/
│       ├── GET.json          # GET /api/users
│       ├── POST.json         # POST /api/users
│       └── [id]/
│           └── GET.json      # GET /api/users/:id  ([dir] = one path segment)
└── GET.html                  # GET /
```

Response files are an optional YAML frontmatter block plus the body:

```yaml
---
status: 201
headers:
  X-Request-Id: abc-123
delay: 100          # milliseconds
---
{"created": true, "id": 42}
```

Run it with `blendwerk ./mocks` (HTTP :8080 + HTTPS :8443 by default) and
test with `curl http://localhost:8080/api/users`.

## Task → Reference

| Task | Read |
|------|------|
| Create/edit mock files, routing rules, frontmatter, content types, pitfalls | [references/mock-structure.md](references/mock-structure.md) |
| Enable/read/analyze request logs; build mocks from observed traffic | [references/request-logs.md](references/request-logs.md) |
| CLI flags, HTTP/HTTPS modes, TLS certificates, Docker | [references/cli.md](references/cli.md) |

## Do / Don't

| Do | Don't | Why |
|----|-------|-----|
| Name files after the method: `GET.json`, `post.json` | Use `index.json`, `response.json`, or any other stem | Only `GET POST PUT DELETE PATCH HEAD OPTIONS` stems (case-insensitive) create routes; everything else is **silently ignored** — no route, no warning |
| Use `[id]` directories for path parameters | Use `:id`, `{id}`, or `*` directories | Only `[name]` is parameter syntax; `:id` becomes a literal segment, and wildcards/catch-alls do not exist |
| Create an explicit `HEAD.json` when clients send HEAD | Expect HEAD to be answered from `GET.json` | HEAD is never derived from GET; without its own file the request gets a 404 |
| Write lowercase frontmatter keys: `status:`, `delay:` | Write `Status:` or misspell a key | Unknown keys are silently ignored, so the response falls back to 200 with no error |
| Close the frontmatter block with a second `---` line | Leave a file starting with `---` unclosed | One broken file fails the whole directory scan: startup aborts; a hot reload keeps the old routes |
| Pick the extension for the Content-Type (`.json`, `.html`, `.txt`) | Expect the extension to affect routing | Extensions only set Content-Type; routing uses the method stem and directories alone |
| Create one static response per (method, path) | Try to vary a response by query string, request body, or headers | Responses are static; query strings don't even participate in route matching |
| Read logged `query`/`body`/`matched_route` as optional keys | Assume every log file has all keys (or `null` values) | Absent values are omitted entirely from the log JSON/YAML |

Also: a defined path with a missing method file returns **404** (not 405), and
request logs land under the **literal** request path
(`request-logs/api/users/42/GET/...`), with `matched_route` inside each file
linking back to the mock pattern.
