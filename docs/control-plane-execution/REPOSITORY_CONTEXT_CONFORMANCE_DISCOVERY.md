# Repository readiness and bounded context-pack discovery (CP-08-92)

**Decision:** build on existing ALTAI contracts; do not introduce another
repository, session, context, credential or evidence store. Package 090's
first implementation is a read-only readiness manifest plus a bounded fixture.

## Canonical ownership inventory

| Candidate input | Canonical owner | Required admission boundary | Decision |
| --- | --- | --- | --- |
| Workspace/project/goal/work text | `run_context` (031) | Workspace resolves project; Work resolves in that project; ancestry is org-scoped | Reuse. The existing 24 KiB bounded, deterministic pack is the only context text source. |
| Attempt/run identity | Attempt and RunBinding repositories (030) | Binding Work and agent must exactly match the immutable Attempt | Reuse. A caller cannot supply a second Work id for a run. |
| Repository URL | RepositoryScope + WorkspaceScopeGate (050) | Workspace project resolves org; explicit `(org, URL)` grant is required | Reuse. An absent workspace binding or grant is denied. |
| Evidence artifact references | EvidenceRepository (045) | Immutable Evidence must be attributed to the canonical Work/Attempt | Reuse as references only. Context cannot ingest artifact bodies or credentials. |
| Activity/correlation trace | Activity repository / replay contracts (035, 060) | Query is org/work scoped and correlation remains attributable | Reference only. No Activity copy or new audit stream. |

## Fixture: `RC-090-readiness-context-v1`

| Phase | Action | Required observation | Failure |
| --- | --- | --- | --- |
| Resolve | Resolve one Attempt-bound Work context through canonical repositories | Context contains only canonical scope/work fields and is within the fixed byte budget | Caller-supplied Work, path or project changes the pack |
| Permit | Ask `WorkspaceScopeGate` for the resolved workspace repository | Exact org/URL grant is returned before any repository material is referenced | Ungranted, unbound or foreign repository is admitted |
| Measure | Record requested and admitted byte counts plus `truncated` from `BoundedRunContext` | Admission has explicit boundedness evidence without persisting a duplicate pack | Context is silently truncated or copied to a second durable store |
| Reference | Attach only Evidence ids/kinds/references for the same Work/Attempt | References remain immutable and attributable; artifact content is not fetched | Foreign Attempt evidence, artifact body, token or credential enters context |
| Replay | Repeat identical canonical reads after repository reopen | Readiness manifest is byte-stable except explicitly excluded runtime presentation fields | Ordering, scope, or reference set changes without canonical record change |

**Pass rule:** the fixture must use only existing repositories; every repository
reference has a permit; context bytes are bounded and measured; and no
credential-like string, artifact body, filesystem path, external session or
new persistent table participates.

## Follow-up boundary

CP-08-93 may add the transport-free manifest and this fixture. It may not
clone/fetch a repository, create an index/vector store, persist context text,
read an Evidence artifact body, or expose workspace credentials.
