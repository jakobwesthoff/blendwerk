# Request Logs

How to enable, locate, read, and analyze the request logs blendwerk writes,
and how to turn observed traffic into new mock files.

## Contents

- [Enabling](#enabling)
- [Directory Layout](#directory-layout)
- [Log File Schema](#log-file-schema)
- [Analysis Recipes](#analysis-recipes)
- [Turning Logs into Mocks](#turning-logs-into-mocks)

## Enabling

```bash
blendwerk ./mocks --request-log ./request-logs
# YAML instead of JSON:
blendwerk ./mocks --request-log ./request-logs --request-log-format yaml
```

Logging is asynchronous and never blocks or delays responses. Every request
is logged, including 404s and 405s.

## Directory Layout

One file per request, under the **literal request path** (not the matched
route pattern), then the method:

```
request-logs/
├── GET/                                  # requests to /
│   └── 2025-01-28T15-29-01.000042Z_01HQKP5H8Y....json
└── api/
    └── users/
        ├── GET/                          # GET /api/users
        │   └── 2025-01-28T15-30-45.123456Z_01HQKP6J9Z....json
        └── 42/
            └── GET/                      # GET /api/users/42
                └── 2025-01-28T15-31-12.456789Z_01HQKP7A1A....json
```

Consequences of the literal-path layout:

- Requests to a parameterized route scatter across one directory per concrete
  value (`api/users/1/GET/`, `api/users/42/GET/`), while the mock lives in a
  single `api/users/[id]/` directory. Use the `matched_route` field inside
  each file to group them back together.
- A 404 to `/api/nonexistent` creates `request-logs/api/nonexistent/GET/...` —
  scanning for directories that have no counterpart in the mock tree reveals
  what clients requested but the mock does not cover.

Filenames are `<timestamp>_<ULID>.<json|yaml>`. The timestamp format is
`YYYY-MM-DDTHH-MM-SS.microsecondsZ` (UTC, colons replaced by dashes), so
plain lexicographic filename sorting is chronological.

## Log File Schema

```json
{
  "metadata": {
    "timestamp": "2025-01-28T15-30-45.123456Z",
    "request_id": "01HQKP6J9Z0000000000000000"
  },
  "request": {
    "method": "GET",
    "uri": "/api/users/42?verbose=1",
    "path": "/api/users/42",
    "query": "verbose=1",
    "headers": {
      "user-agent": "curl/8.0.0",
      "accept": "*/*"
    },
    "body": "...",
    "matched_route": "/api/users/:id"
  },
  "response": {
    "status": 200,
    "headers": {
      "content-type": "application/json"
    },
    "body": "{\"id\": 42, \"name\": \"Alice\"}",
    "delay_ms": 0
  }
}
```

Field notes:

- `path` is the literal request path; `matched_route` is the pattern that
  served it, with parameters in `:name` form.
- `query`, `request.body`, and `matched_route` are **omitted entirely** when
  absent (empty body, no query string, 404/405 with no matching route) — they
  are not present as `null`. Scripts must treat these keys as optional.
- Header values that are not valid UTF-8 appear as `<binary>`.
- `response.body` is the full body as a string; `delay_ms` is the configured
  frontmatter delay.

## Analysis Recipes

All examples assume JSON logs and `jq`.

```bash
# All requests that got a 404 (i.e. traffic the mock doesn't cover):
find request-logs -name '*.json' \
  -exec jq -r 'select(.response.status == 404) | "\(.request.method) \(.request.path)"' {} + \
  | sort | uniq -c | sort -rn

# Everything that hit one route pattern, in chronological order:
find request-logs -name '*.json' | sort \
  | xargs jq -r 'select(.request.matched_route == "/api/users/:id") | .request.path'

# Bodies clients POSTed to an endpoint (to design a realistic mock response):
find request-logs/api/users/POST -name '*.json' \
  -exec jq -r '.request.body // empty' {} +

# Distinct query strings seen on a path:
find request-logs/api/users/GET -name '*.json' \
  -exec jq -r '.request.query // empty' {} + | sort -u
```

## Turning Logs into Mocks

Workflow to cover a request that currently 404s:

1. Find the gap: filter logs for `response.status == 404` (recipe above) and
   note `request.method` and `request.path`.
2. Translate the path into directories under the mock root. Replace segments
   that are variable (IDs, slugs — several sibling log directories for the
   same position are the giveaway) with a `[param]` directory.
3. Create `<METHOD>.json` in that directory. Use the logged `request.body`
   and query values to decide what a realistic response body looks like.
4. Add frontmatter only if the defaults (status 200, no extra headers, no
   delay) are wrong.
5. blendwerk hot-reloads automatically; replay the request and confirm the
   log now shows the expected status and a `matched_route`.

Example: logs show `POST /api/orders/42/cancel` returning 404.

```bash
mkdir -p mocks/api/orders/[orderId]/cancel
```

```yaml
# mocks/api/orders/[orderId]/cancel/POST.json
---
status: 202
---
{"status": "cancelling"}
```

File format details (frontmatter fields, content types) are in
[mock-structure.md](mock-structure.md).
