# doctool

Documentation tooling for the [lomi.](https://lomi.africa) monorepo. Run it as **`dt`**.

**Repository:** [github.com/lomiafrica/doctool](https://github.com/lomiafrica/doctool) — standalone project (not a submodule of lomi).

When working inside the lomi monorepo, clone this repo to `apps/doctool`:

```bash
git clone git@github.com:lomiafrica/doctool.git apps/doctool
```

Built on a portable fork of the Composer `code_intel` engine (tree-sitter scan, hybrid search, cross-encoder rerank) with lomi-specific connectors for OpenAPI, MDX, SDK manifests, and competitor reference docs.

**Linear:** [CORE-38](https://linear.app/lomi/issue/CORE-38/build-ai-powered-developer-documentation-cli-tool)

## Install

From the monorepo root:

```bash
cd apps/doctool
cargo build --release
cargo install --path crates/doctool-cli   # installs `dt` on PATH
```

Or run without installing:

```bash
cargo run -p doctool-cli -- scan --root /path/to/lomi.
```

## Commands

| Command | Description |
| --- | --- |
| `dt scan` | Index code roots, OpenAPI, MDX, SDK manifest, competitor docs → `.doctool/` |
| `dt check` | Run `pnpm lint` + `pnpm docs:drift` + `pnpm screenshots:verify` in `apps/docs` (CI gate) |
| `dt drift` | Rust-native drift report + optional TS `docs-drift.ts` |
| `dt graph` | Export operations ↔ guides ↔ SDK knowledge graph JSON |
| `dt scaffold` | `CONFIRM_BOOTSTRAP=1 pnpm run api:regenerate-rest-reference` |
| `dt sync-i18n` | Deterministic i18n gap/stale/structure checks + lock refresh (`--check`, `--dry-run`, `--lock`) |
| `dt translate-i18n` | LLM incremental segment translation for `.fr.mdx` siblings (`--check`, `--dry-run`, `--force`) |
| `dt improve` | Improve MDX prose with RAG context (`--path`, `--stdout`, `--output`) |
| `dt diff` | Unified diff vs canonical MDX (`--path`, `--proposed`) |
| `dt suggest` | Scan codebase + drift, merge executable commands with LLM plan (`--skip-ai`, `--skip-ts`) |
| `dt doctor` | Check monorepo setup, i18n.lock, pnpm, and first-run notes |

### Flags

- `--root <path>` — monorepo root (auto-detected via `apps/docs/package.json`)
- `--config <path>` — `doctool.config.toml` override
- `--json` — machine-readable output

### LLM configuration

Set `DOCTOOL_LLM_API_KEY` (or `OPENAI_API_KEY`) for `translate-i18n` and `improve`. Use `DOCTOOL_LLM_MOCK=1` or `provider = "mock"` in config for CI.

See `[llm]` in [`doctool.config.toml`](./doctool.config.toml).

## Configuration

Default paths live in [`doctool.config.toml`](./doctool.config.toml). Cache output:

- `.doctool/index.json` — full scan snapshot
- `.doctool/graph.json` — knowledge graph

## lomi CLI integration

Maintainer-only subcommands delegate to `dt`:

```bash
lomi docs check
lomi docs scan
lomi docs drift
lomi docs graph
lomi docs scaffold
lomi docs sync-i18n
lomi docs translate-i18n
lomi docs improve --path build/usage-billing.mdx --stdout
lomi docs diff --path build/usage-billing.mdx
```

Build doctool first (`cargo build` in `apps/doctool`) or install the `dt` binary on `PATH`.

## Prerequisites

- Rust 1.75+
- `pnpm` (for `check` / `scaffold`)
- `apps/design` submodule initialized for competitor corpus indexing:

```bash
git submodule update --init apps/design
```

## Architecture

```
crates/doctool-core/   # library (intel engine + lomi connectors)
crates/doctool-cli/    # `dt` binary
```

Copied from Composer (`composer/src-tauri/src/code_intel/`): `loader`, `parser`, `indexer`, `global_index`, `graph`, `search`, `embedder`, `vector_store`, `language_detect`, `rerank`.

## v1.1 (planned)

- `dt improve` — LLM prose suggestions with RAG over scan index + competitors
- `dt diff` — unified diff vs canonical MDX
- MCP server wrapping `doctool-core`

See [`apps/design/docs/note.md`](../design/docs/note.md) for the full product plan.
