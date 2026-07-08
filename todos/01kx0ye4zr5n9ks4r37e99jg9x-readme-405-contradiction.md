# README/code contradiction: 405 for missing method on existing path

## Observation

`README.md` (Error Responses section) documents:

> - `404 Not Found` — No route matches the path
> - `405 Method Not Allowed` — Path exists but method isn't defined

The code does not implement the 405 case. `find_matching_route`
(`src/server.rs:242-248`) matches method **and** path together
(`r.method == method && r.matches(path)`); when nothing matches, the handler
returns `not_found` (`src/server.rs:271-274`), so a request like
`POST /api/users` against a tree that only defines `GET.json` gets
`404 Route not found: POST /api/users`.

A 405 is only ever returned for HTTP methods blendwerk does not support at
all: `parse_http_method` (`src/server.rs:228-239`) returns `None` for
anything outside GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS (e.g. TRACE,
CONNECT), which triggers `method_not_allowed` (`src/server.rs:257-264`).

## Task

Decide which side is authoritative:

- **Docs wrong:** remove/correct the 405 bullet in `README.md` (and the
  derived gh-pages documentation, which is generated from the README).
- **Code bug:** implement real 405 semantics — when no (method, path) route
  matches but some other method matches the same path, return 405 (RFC 9110
  also expects an `Allow` header listing the permitted methods).

## Affected if the decision is "code bug"

- `src/server.rs` handler/matching logic.
- The shipped agent skill documents the current 404 behavior as fact:
  `skills/blendwerk/SKILL.md` (Do/Don't section) and
  `skills/blendwerk/references/mock-structure.md` (Error Responses section)
  must be updated together with the fix.
- Request log assertions: a 405 response would then carry a `matched_route`
  or not, depending on implementation choice.

## Related observation (same session)

`README.md` shows a request log example with `"body": null`, but the
serializer omits absent fields entirely (`skip_serializing_if` on `query`,
`body`, `matched_route` in `src/request_logger.rs:122-133`). Minor doc
inaccuracy, fix alongside whichever direction is chosen.
