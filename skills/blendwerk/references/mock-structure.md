# Mock Directory Structure

How blendwerk maps a directory tree to HTTP routes, and the exact format of
response files. This is the authority for creating or editing mocks.

## Contents

- [Directory Layout → Routes](#directory-layout--routes)
- [Supported Methods](#supported-methods)
- [Path Parameters](#path-parameters)
- [Route Matching Rules](#route-matching-rules)
- [Response File Format](#response-file-format)
- [Content-Type Inference](#content-type-inference)
- [Error Responses](#error-responses)
- [Hot Reload](#hot-reload)
- [Dos and Don'ts with Examples](#dos-and-donts-with-examples)
- [Pitfalls](#pitfalls)

## Directory Layout → Routes

Folders are URL path segments; the file name (without extension) is the HTTP
method. The extension only sets the Content-Type, never the routing.

```
mocks/
├── GET.html                  # GET /
├── api/
│   ├── health/
│   │   └── GET.json          # GET /api/health
│   └── users/
│       ├── GET.json          # GET /api/users
│       ├── POST.json         # POST /api/users
│       └── [id]/
│           ├── GET.json      # GET /api/users/:id
│           ├── PUT.json      # PUT /api/users/:id
│           └── DELETE.json   # DELETE /api/users/:id
```

## Supported Methods

Exactly these file stems create routes (case-insensitive, so `GET.json`,
`get.json`, and `Get.json` are equivalent):

`GET` `POST` `PUT` `DELETE` `PATCH` `HEAD` `OPTIONS`

Any other file name is silently ignored during the scan. This means a
`README.md` or `.gitkeep` inside the mock tree is harmless, but it also means
a typo like `GETT.json` or `INDEX.json` produces no route and no warning.

`HEAD` is not derived from `GET`. If a client sends HEAD requests, create an
explicit `HEAD.json` file, otherwise the request gets a 404.

## Path Parameters

A directory named `[param]` matches exactly one path segment of any value:

```
mocks/api/users/[userId]/posts/[postId]/GET.json
# → GET /api/users/:userId/posts/:postId
```

There is no wildcard or catch-all: a route matches only when the request has
exactly the same number of segments. `mocks/api/users/[id]/GET.json` matches
`/api/users/42` but not `/api/users` or `/api/users/42/extra`.

## Route Matching Rules

- **First match wins.** Routes are tried in the order the directory scan
  discovered them; that order is not configurable. Static directories
  (`admin/`) and parameter directories (`[id]/`) have equal priority, so do
  not rely on a static route shadowing a parameter route next to it.
- **Query strings are ignored for matching.** `/api/users`, `/api/users?page=2`,
  and `/api/users?limit=10` all hit the same mock file. Query parameters do
  appear in request logs (see [request-logs.md](request-logs.md)).
- **Responses are static.** A (method, path) pair always returns the same
  response; the request body and headers cannot influence it.

## Response File Format

Optional YAML frontmatter between `---` delimiters, then the response body:

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

All frontmatter fields are optional; a file may also be pure body with no
frontmatter at all.

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `status` | integer | 200 | HTTP status code |
| `headers` | map | `{}` | Response headers; may override the inferred Content-Type |
| `delay` | integer | 0 | Milliseconds to wait before responding |

Empty frontmatter (`---` immediately followed by `---`) is valid and yields
all defaults. An empty body (e.g. for a 204) is valid too:

```yaml
---
status: 204
---
```

## Content-Type Inference

Derived from the file extension; override via `headers: {Content-Type: ...}`:

| Extension | Content-Type |
|-----------|--------------|
| `.json` | `application/json` |
| `.html`, `.htm` | `text/html` |
| `.xml` | `application/xml` |
| `.txt` | `text/plain` |
| `.css` | `text/css` |
| `.js` | `application/javascript` |
| anything else | `application/octet-stream` |

## Error Responses

- **404 Not Found** (`Route not found: METHOD /path`) is returned both when no
  route matches the path **and** when the path exists but the requested method
  has no file. There is no automatic 405 for a missing method on an existing
  path.
- **405 Method Not Allowed** is returned only for methods blendwerk does not
  support at all (e.g. `TRACE`, `CONNECT`).

## Hot Reload

The mock directory is watched recursively. Create, modify, and remove events
trigger a full rescan after a 100 ms debounce; no restart is needed. If the
rescan fails (e.g. a file with broken frontmatter), the error is logged and
the previously loaded routes stay active. At **startup**, the same error
aborts the server instead.

## Dos and Don'ts with Examples

**Path parameters use `[name]` — nothing else is parameter syntax.**

```bash
# ✅ Do
mkdir -p 'mocks/api/users/[id]'        # GET /api/users/:id

# ❌ Don't — ':id' and '{id}' become literal static segments,
#            so only the exact URL /api/users/:id would match
mkdir -p 'mocks/api/users/:id'
mkdir -p 'mocks/api/users/{id}'
```

Quote `[id]` in shell commands: unquoted brackets are glob characters.

**The file stem must be an HTTP method.**

```bash
# ✅ Do — one file per method
mocks/api/users/GET.json
mocks/api/users/POST.json

# ❌ Don't — silently ignored, creates no route and no warning
mocks/api/users/index.json
mocks/api/users/users.json
```

**Frontmatter keys are lowercase and exact.**

```yaml
# ✅ Do
---
status: 404
delay: 500
---
{"error": "not found"}
```

```yaml
# ❌ Don't — 'Status' and 'dealy' are unknown keys, silently ignored;
#            this responds with 200 and no delay
---
Status: 404
dealy: 500
---
{"error": "not found"}
```

**Empty-body responses still close the frontmatter block.**

```yaml
# ✅ Do — a 204 with no body
---
status: 204
---
```

```yaml
# ❌ Don't — unclosed frontmatter is a parse error that fails the
#            entire directory scan
---
status: 204
```

**Override Content-Type through headers, not an invented extension.**

```yaml
# ✅ Do — mocks/api/legacy/GET.json
---
headers:
  Content-Type: application/vnd.api+json
---
{"data": {"type": "users", "id": "1"}}
```

```bash
# ❌ Don't — an unknown extension serves application/octet-stream
mocks/api/legacy/GET.vndjson
```

**One response per (method, path) — model variants as separate paths.**

```bash
# ✅ Do — an error case gets its own route
mocks/api/users/GET.json               # the success case
mocks/api/users-error/GET.json         # status: 500, for error-path tests

# ❌ Don't — there is no way to switch a response by query string,
#            request body, or headers; ?fail=1 hits the same file
```

## Pitfalls

- **Out-of-range `status` silently becomes 200.** A value that is not a valid
  HTTP status code (e.g. `status: 99`) is served as 200. Together with the
  ignored-unknown-keys behavior above, this makes "mock unexpectedly returns
  200" almost always a frontmatter problem.
- **Invalid header names/values are silently dropped.** A header that is not
  a valid HTTP header name/value simply does not appear in the response.
- **Text only.** Response files are read as UTF-8 text; binary bodies
  (images, PDFs) are not supported.
- **Everything is in memory.** All response files are loaded at startup and
  on each reload; avoid very large bodies.
