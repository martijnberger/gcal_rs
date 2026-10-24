# Google Calendar API completeness plan

## Goal and source of truth

Implement the full Calendar v3 resource surface and repair existing request/model correctness. Use Google's discovery schema (revision 20261021) and official method documentation: https://www.googleapis.com/discovery/v1/apis/calendar/v3/rest . Preserve existing named clients and re-exports where their contracts remain correct; migrate incompatible write contracts explicitly, without deprecated shims. No real credentials or calendar mutations are needed for deterministic local verification.

## Delivery stack

Four phase bookmarks and PRs in `martijnberger/gcal_rs`:

1. `calendar-api/phase-1-correctness` targets `main`.
2. `calendar-api/phase-2-models-pagination` targets phase 1.
3. `calendar-api/phase-3-sync-patch` targets phase 2.
4. `calendar-api/phase-4-resources` targets phase 3.

Prior dependency/OAuth security updates are included in phase 1. Unrelated local agent instructions remain outside the published stack. Each phase records its verification in its PR; PR descriptions disclose agent authorship. Do not merge the PRs automatically.

## Phase 1 — transport and write correctness

- Preserve patched dependencies and OAuth state/PKCE, credential-redaction, loopback callback, and refresh-token fixes.
- Return structured Google API errors retaining HTTP status, reason, message, and details; accept all successful status codes, including 204. Preserve reqwest transport errors.
- Send JSON with its content type; support bodyless POST operations and conditional mutation headers without redirecting credentials.
- Encode path identifiers individually; correct event collection/action endpoints, quick-add calendar selection, and move notification enum handling.
- Correct `iCalUID` and `guestsCanInviteOthers` wire names.
- Introduce explicit writable event payloads, omitting response-only/default-injected properties. Distinguish omission from explicit false/empty values. Keep read models out of write serialization.
- Verify real client behavior against local HTTP fixtures with injected test transport/base URLs; test error envelopes, 204, JSON headers, endpoint paths, and omission/false semantics. Smoke-run the changed public path.

## Phase 2 — schema fidelity and pagination

- Complete Event and nested conference, attendee, attachment, working-location, focus-time, out-of-office, birthday, extended, and label fields against discovery. Fix SIP spelling, optional nested fields, integer widths, public conference access, and conference solution name placement.
- Complete Calendar, CalendarListEntry, Settings and list envelope fields; support `writerWithoutPrivateAccess` and sparse/deleted entries.
- Preserve unknown future enum values where practical without losing known ergonomic variants.
- Represent repeated query keys without overwriting values, and expose typed list options plus raw query escape hatches.
- Expose page-returning methods and collect every page in convenience list methods, including recurring instances. Retain calendar context on returned events.
- Verify empty intermediate pages, multi-page reads, sparse official payloads, round-trip model fidelity, repeated filters, and calendar context. Update examples/docs for changed contracts.

## Phase 3 — synchronization and safe patching

- Expose event, calendar-list, and settings incremental synchronization with `syncToken`, all continuation pages, and final `nextSyncToken`.
- Reject incompatible event sync filters; preserve the same sync query throughout pagination, including deleted resources. Surface 410 as an explicit full-resync requirement; never erase application storage automatically.
- Implement typed event PATCH with explicit omitted/set/null semantics and optional ETag `If-Match` for updates/deletes/patches. Surface 412 conflicts normally.
- Carry notification, attachment, and conference-data-version options through write operations; enable conference data version 1 when writing conference data.
- Verify multi-page deltas, empty pages, deletions, expired sync tokens, filter rejection before network, explicit clears, and concurrent-update preconditions. Document safe full-resync behavior.

## Phase 4 — remaining resource surface

- Implement all discovery methods for calendars (get/insert/update/patch/delete/clear/transferOwnership), calendarList (get/insert/update/patch/delete/watch plus list), settings (get/list/watch), ACL (get/list/insert/update/patch/delete/watch), colors.get, freebusy.query, events.watch, and channels.stop.
- Provide typed request/response and option models, per-method path/query/body behavior, public constructors/accessors, and factories on GCalClient. Preserve existing event method names where usable; return resource responses when the API provides them.
- Implement typed notification Channel creation/stop and explain that applications own webhook authentication/resource-state handling and channel renewal.
- Implement Calendar batch transport at `/batch/calendar/v3`: multipart request encoding with unique Content-ID values, per-item status/error results, robust boundary parsing, conditional headers, and Google's 1000-operation limit. Batch writes must use the same safe payload contracts as ordinary requests.
- Verify representative resource workflows through actual client calls against deterministic HTTP fixtures, every discovery method's verb/path/body/query contract, ownership transfer, freebusy partial errors, watch/stop, and mixed-success multipart batches. No fake network implementations or skipped methods.

## Acceptance and verification

- All 38 discovery methods have typed callable support, not just data structures or generic HTTP escape hatches.
- Existing examples compile after migration; docs demonstrate writes, pagination/sync, new resources, notifications and batch use.
- Retain deterministic tests for consumer-visible behavior and uncertain boundaries, not source-text/wiring/wording assertions.
- Each phase: smoke-run changed public behavior, `cargo test --locked --all-targets`, `cargo test --locked --doc`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, and formatting checks. Run advisory checks after dependency changes and once at completion.
- Real authenticated Google Calendar writes are not performed without credentials and an explicitly designated test calendar; report that limit rather than claiming live integration validation.

## Implementation record

### Phase 1 completed

Structured HTTP/API errors preserve statuses and Google reasons/details; successful empty responses work. JSON writes use explicit `EventWrite` payloads, and quick-add/move are bodyless with correct encoded endpoints. Public event reads retain calendar context. Dependency/OAuth hardening and a tracked lockfile are included.

Verification: 16 tests passed; the public rustdoc example compiled; strict Clippy and advisory checks passed. An executable smoke made a read-only Google request using an intentionally invalid token and observed a structured HTTP 401, and verified safe write omission/false semantics. No authenticated calendar changes were made.

### Phase 2 completed

Completed discovery event/nested/calendar/settings fields, lossless unknown enum variants, sparse response handling, public conference metadata, repeated query pairs, and page/all-page APIs. Convenience event/calendar lists and recurring instances now traverse empty intermediate pages and preserve final sync tokens and event calendar context.

Verification: 34 tests passed, public rustdoc compiled, strict Clippy passed. An executable smoke round-tripped sparse/future official-shaped payloads, encoded repeated filters, and observed a structured real Google 401 from the new page API. No authenticated mutations were made.

Phases 3–4 remain to be implemented in the subsequent stack changes.
