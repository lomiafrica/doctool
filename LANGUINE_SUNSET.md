# Languine sunset audit

This document records what was taken from [Languine](https://github.com/languine-ai/languine) into doctool, what was skipped, and the deletion checklist.

**Status:** v1.1 ports complete when every **TAKE** row below is marked done.

## ALREADY_HAVE (no port needed)

| Languine | doctool | Notes |
|----------|---------|-------|
| `packages/cli/src/utils/lock.ts` | `crates/doctool-core/src/i18n/lock.rs` | Segment-level MD5, YAML lock |
| Incremental stale detection | `i18n/sync.rs` + `locale_stale` | Plus structure, links, orphans |
| `utils/path.ts` (`.locale.` suffix) | `sources/i18n/locale.rs` | `.fr.mdx` siblings |
| `parsers/formats/markdown.ts` | `sources/mdx/document.rs` | Segment model (prose, headings, code, JSX) |
| CI translate gate | `.github/workflows/doctool.yml` | `dt sync-i18n --check` |
| `lomi docs` delegation | `apps/cli/src/commands/docs_cmd.rs` | sync-i18n wired |

## SKIP (wrong domain — do not port)

| Languine | Reason |
|----------|--------|
| 15+ app parsers (JSON, PO, ARB, Xcode, …) | Docs use MDX only |
| `commands/init`, `auth`, `locale`, `overrides` | SaaS onboarding |
| `commands/transform.ts` (jscodeshift) | App UI string extraction |
| `commands/sync.ts` (KV key deletion) | MDX page model differs |
| `apps/web` (Workflows, Postgres, tRPC) | No hosted doctool server |
| `packages/action` GitHub Action | CLI-only; CI uses `sync-i18n --check` |
| `utils/api.ts`, `session.ts` | Languine cloud API |
| `utils/sse.ts` | No remote worker; stderr progress instead |

## TAKE (ported into doctool)

| Languine source | doctool module | Status |
|-----------------|----------------|--------|
| `workflows/utils/chunk.ts` | `llm/chunk.rs` | done |
| `workflows/utils/tokeniser.ts` | `llm/tokeniser.rs` | done |
| `workflows/utils/prompt.ts` | `llm/prompt.rs` | done |
| `workflows/utils/translate.ts` | `llm/client.rs` | done |
| `workflows/translate-locale.ts` (chunk loop) | `i18n/translate.rs` | done |
| `commands/translate.ts` (incremental flow) | `dt translate-i18n` | done |
| `utils/git.ts` | `provenance.rs` | done |
| — | `improve/` (CORE-38, not in Languine) | done |
| — | `diff/` (`similar` crate) | done |

## Deletion checklist

Before removing `/Users/babacar/Projects/languine`:

1. [x] `cargo test --workspace` passes in `apps/doctool`
2. [x] `dt sync-i18n --check` passes on monorepo (or only known issues)
3. [x] `dt translate-i18n --dry-run` flags pending segments
4. [x] `dt improve --path ... --stdout` produces reviewable output (use `DOCTOOL_LLM_MOCK=1` without piping for MDX stdout)
5. [x] `dt diff` produces unified patch
6. [x] No `languine` references in lomi monorepo (grep `apps/doctool`)
7. [ ] Optional: `git clone --mirror` archive of Languine repo
8. [x] Remove local clone (`/Users/babacar/Projects/languine` deleted)
