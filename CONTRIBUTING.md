# Contributing to doctool

Maintainer tooling for documentation drift, indexing, i18n, and AI-assisted prose.

## Local development

```bash
cd apps/doctool
cargo build
cargo test --workspace
./scripts/smoke-test.sh   # optional: tests + fixture scan/drift + release build
```

See **[TESTING.md](./TESTING.md)** for the full test pyramid, fixture layout, coverage map, and v1.1 roadmap.

Run from monorepo root:

```bash
./apps/doctool/target/debug/dt scan --root .
./apps/doctool/target/debug/dt drift --root . --skip-ts
./apps/doctool/target/debug/dt check --root .
```

## Project layout

| Path | Purpose |
| --- | --- |
| `crates/doctool-core/src/intel/` | Composer `code_intel` fork (scan, parse, search) |
| `crates/doctool-core/src/sources/` | OpenAPI, MDX, SDK, competitors connectors |
| `crates/doctool-core/src/drift/` | Rust-native drift categories |
| `crates/doctool-core/src/i18n/` | Lock file, deterministic sync, LLM translate |
| `crates/doctool-core/src/llm/` | OpenAI-compatible client, chunking, prompts, mock provider |
| `crates/doctool-core/src/improve/` | RAG-backed MDX improve |
| `crates/doctool-core/src/diff/` | Unified MDX diff (`similar` crate) |
| `crates/doctool-core/src/graph/` | Knowledge graph export |
| `crates/doctool-cli/` | CLI binary |

## Syncing from Composer

When updating the intel engine, copy portable files from `composer/src-tauri/src/code_intel/`:

- `types.rs`, `utils.rs`, `loader.rs`, `parser.rs`, `indexer.rs`
- `global_index.rs`, `graph.rs`, `search.rs`
- `embedder.rs`, `vector_store.rs`, `language_detect.rs`

Also `composer/src-tauri/src/db/rerank.rs` → `crates/doctool-core/src/rerank.rs`.

Do **not** copy Tauri-coupled modules (`agent.rs`, `ai/*`, `history.rs`, etc.).

## CI

`cargo test --workspace` and `dt check` run on PRs that touch `apps/api`, `apps/docs`, `apps/mcp`, or `apps/doctool` (see `.github/workflows/doctool.yml`).

## Related docs

- [CORE-38 design brief](../design/docs/note.md)
- [Maintaining CLI and MCP](../docs/content/docs/resources/contributing/maintaining-cli-mcp.mdx)
