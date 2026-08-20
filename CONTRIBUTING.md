# Contributing

HistoryWorld aims for Wikipedia-like collaboration, but visual plausibility must never erase the
difference between evidence and invention.

The project license and third-party content policy have not yet been selected. Do not submit
externally licensed historical assets until that decision and an attribution pipeline are in place.

## Code changes

Install the pinned text and schema tooling once, then use the repository targets:

```bash
make setup
make ci
```

`make format` applies Rustfmt, Prettier, and Buf. `make format-unreal` applies the root ClangFormat
policy to Unreal C/C++ when `clang-format` is available. It is separate because lightweight CI does
not include an Unreal engine toolchain; the production engine lane must pin the exact build and
toolchain before cooking. Generated Unreal headers must remain the final include, so the formatter
never sorts includes.

Formatting and linting versions are repository-pinned through `rust-toolchain.toml`,
`package-lock.json`, and the Buf/Actionlint versions in `Makefile`. Do not replace a pinned check
with an unversioned global download.

VS Code recommendations live in `.vscode/`. Zed project settings and tasks live in `.zed/`; after
you trust the worktree, Zed installs the small set of language extensions declared there. Both
editors keep background Rust checks to `cargo check --locked`; the complete Clippy/schema/test suite
runs explicitly through `make ci` to avoid spending CPU on every save.

Changes to canonical types, hashing, ordering, time, cells, protocols, PRNGs, or materialization
need an ADR/schema version, compatibility fixture, and migration story. Never change an existing
field/tag's meaning in place.

Performance changes must state their expected CPU, memory, network/storage, and where relevant
GPU/VRAM impact. New caches need byte limits, invalidation keys, metrics, and a reason. New loops
and queues need hard admission limits, cancellation, and backpressure.

Published database migrations are append-only records. Add a new migration for schema changes; do
not bulk-format or edit a migration that has shipped. SQL is kept out of the automatic formatter for
that reason.

## Historical content

Contributions should eventually arrive as immutable source/citation/claim revisions in a change set,
not direct edits to a rendered world. Each assertion needs:

- source revision and precise locator;
- rights/attribution metadata;
- original date expression plus uncertainty;
- original coordinates/reference frame plus accuracy;
- epistemic class;
- rationale for reconstruction/model choices.

Observation, primary report, scholarly inference, model derivation, plausible reconstruction, and
artistic ambient content are distinct. Conflicts become explicit hypotheses. Upvotes are
discovery/review signals, not a truth overwrite.

Automated or new contributors may propose change sets, but publication requires review under a
versioned policy. Corrections append revisions; they do not erase the audit trail.
