# doctool testing strategy

This document explains **what we test today**, **how to run tests**, and **where we are going** (v1 → v1.1 → Phase 3).

## North star

Doctool is the Rust documentation intelligence layer for the lomi. monorepo. Tests must prove:

1. **Deterministic connectors** — OpenAPI, MDX, SDK, competitors parse correctly.
2. **Drift detection** — Rust categories align with (and extend) `apps/docs` TypeScript drift.
3. **Scan + snapshot** — Code intel indexing and `.doctool/` cache round-trip.
4. **CLI contract** — `dt ` and `lomi docs *` behave predictably in CI and locally.

LLM-powered `improve` (v1.1) and review UI (Phase 3) are **out of scope** for current tests.

---

## Test pyramid

```
                    ┌─────────────────────┐
                    │  CI: dt check │  ← full monorepo + pnpm lint/drift
                    └──────────┬──────────┘
                               │
              ┌────────────────┴────────────────┐
              │  CLI smoke (cli_smoke.rs)       │  ← binary + fixture root
              └────────────────┬────────────────┘
                               │
        ┌──────────────────────┴──────────────────────┐
        │  Integration tests (fixture mini-monorepo)   │  ← drift, scan, graph
        └──────────────────────┬──────────────────────┘
                               │
   ┌───────────────────────────┴───────────────────────────┐
   │  Unit tests (intel/, connectors, config, drift helpers)  │
   └─────────────────────────────────────────────────────────┘
```

| Layer | Location | Runs in CI | Purpose |
|-------|----------|------------|---------|
| Unit | `doctool-core/src/**` `#[cfg(test)]` | Yes (`cargo test`) | Fast, isolated logic |
| Integration | `doctool-core/tests/*.rs` | Yes | End-to-end on **fixture** tree (no prod docs mutation) |
| CLI smoke | `doctool-cli/tests/cli_smoke.rs` | Yes | Spawn real binary against fixture |
| Local smoke | `scripts/smoke-test.sh` | No (dev) | One command before PR |
| E2E gate | `.github/workflows/doctool.yml` | Yes | `dt check` on real monorepo |

---

## Fixture mini-monorepo

Path: `crates/doctool-core/tests/fixtures/mini-monorepo/`

A **minimal fake monorepo** with intentional drift issues:

| Category | Fixture setup |
|----------|----------------|
| `missing_endpoint` | `POST /refunds` in OpenAPI allowlist, no MDX page |
| `orphan_doc` | `GET /orphan/only-in-docs` documented but not public |
| `locale_gap` | `build/guides/getting-started.mdx` without `.fr.mdx` |
| `locale_stale` | `build/guides/stale-en.mdx` vs `.doctool/i18n.lock` |
| `locale_structure` | Extra H2 in `stale-en.fr.mdx` vs EN |
| `locale_orphan` | `orphan-fr-only.fr.mdx` with no EN sibling |
| `guide_dead_link` | Link to `/api/missing/MissingController_action` |
| `sdk_unmentioned` | `neverMentionedInDocs` not referenced in MDX corpus |

Also includes: sample TS for code scan, one competitor markdown file, `apps/docs/package.json` for root detection.

**Do not** point CI drift at this fixture for pass/fail — production `dt check` uses the real repo.

---

## Commands

```bash
# All Rust tests (recommended)
cd apps/doctool && cargo test --workspace

# Integration tests only
cargo test -p doctool-core --test drift_integration
cargo test -p doctool-core --test sources_integration
cargo test -p doctool-core --test i18n_integration

# CLI smoke
cargo test -p doctool-cli --test cli_smoke

# i18n sync (fixture — expect non-zero)
./target/debug/dt --root crates/doctool-core/tests/fixtures/mini-monorepo \
  --config crates/doctool-core/tests/fixtures/mini-monorepo/doctool.config.toml \
  sync-i18n --check || true

# Local smoke script (tests + fixture scan/drift + release build)
./apps/doctool/scripts/smoke-test.sh

# Full CI gate locally (needs pnpm in apps/docs)
./apps/doctool/target/release/dt check --root .
```

---

## Coverage map (v1)

| Module | Unit | Integration | Notes |
|--------|------|-------------|-------|
| `intel/*` | ✅ (Composer heritage) | via `engine_scan` | Parser, indexer, search |
| `config` | ✅ `lib.rs` | ✅ root detection | |
| `sources/openapi` | partial | ✅ | |
| `sources/mdx` | — | ✅ | Add unit tests for frontmatter edge cases |
| `sources/sdk` | — | ✅ | |
| `sources/competitors` | — | via scan | |
| `drift/report` | `merge_ts_errors` | ✅ all 5 categories | |
| `graph` | — | ✅ | |
| `snapshot` | — | ✅ save/load | |
| `doctool-cli` | — | ✅ cli_smoke | |
| `pnpm` delegation | — | ❌ | Mocked in v1.1 or CI-only |
| Real monorepo drift | — | CI only | Known issue: `usage-billing.mdx` dead link |

---

## Roadmap

### v1 (current) — Done / in progress

- [x] Fixture mini-monorepo
- [x] Drift integration tests (all Rust categories)
- [x] Scan + snapshot round-trip test
- [x] CLI smoke tests
- [x] `cargo test` in CI
- [ ] Golden JSON snapshots for drift report shape (optional, reduces brittle string asserts)
- [ ] Fix or exclude known prod dead link so `dt check` is green

### v1.1 — `improve`, diff, i18n

- [ ] Test harness for LLM commands (mock provider, no network in CI)
- [ ] `sync-i18n` diff tests against fixture locale pairs
- [ ] MCP server contract tests (stdio JSON-RPC)

### Phase 3 — Review UI

- [ ] Playwright or component tests for drift review surface
- [ ] Visual regression for graph explorer (if shipped)

---

## Adding a new drift category

1. Add variant to `drift/categories.rs` + `as_str()`.
2. Implement detection in `drift/report.rs`.
3. Extend fixture with **one minimal file** that triggers the issue.
4. Add assertion in `tests/drift_integration.rs`.
5. Document in this file and `CONTRIBUTING.md`.

---

## Related

- Linear: [CORE-38](https://linear.app) (doctool)
- Design brief: `apps/design/docs/note.md`
- TS drift reference: `apps/docs/lib/scripts/docs-drift.ts`
- Competitor corpus: `apps/design/docs/competitors/`
