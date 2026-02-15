# Agent Guidelines for Pupu Project

This document provides guidelines for AI agents working on the Pupu codebase. It covers build commands, linting, testing, code style, and project conventions.

## Build Commands

### Prerequisites

- [Bun](https://bun.sh) (JavaScript runtime)
- [Rust](https://rust-lang.org) (with `wasm32-unknown-unknown` target)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites)
- [uv](https://docs.astral.sh/uv/) (Python tool)
- [Android SDK](https://developer.android.com/studio) (for Android builds)

### Initialization

```bash
bun run install:pre   # Build WASM endpoint
bun install           # Install npm dependencies
bun run install:post  # Generate icons, IPC bindings, database schema, install Playwright
```

### Development

```bash
# Web development
bun run dev

# Native desktop development (Tauri)
bun run native:dev

# Android development
bun run android:dev
```

### Production Builds

```bash
# Web
bun run build

# Native desktop
bun run native:build

# Android APK
bun run android:build
```

### Web Preview

```bash
bun run preview   # Runs wrangler dev on Cloudflare Workers
```

### WASM Build

```bash
bun run wasm:build
```

## Linting and Testing

### TypeScript/JavaScript

```bash
# Type checking and ESLint
bun run check

# Run Playwright end‑to‑end tests
bun run test

# Run a single Playwright test file
bun run test tests/auth.test.ts

# Run tests matching a title pattern
bun run test -- --grep "登录表单验证"
```

### Rust

```bash
# Format check
cargo fmt --check

# Linting with Clippy
cargo clippy --workspace

# Run all Rust unit tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p utils

# Build in release mode
cargo build --release
```

## Code Style Guidelines

### Naming Conventions

- **Functions and variables**: `snake_case` (applies to both Rust and TypeScript)
- **Class names**: `PascalCase`
- **Interface/trait names**: `PascalCase`
- **Module names**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Type parameters**: `T`, `U`, `V` or descriptive camelCase (`TResult`)

### TypeScript / SolidJS

- Use strict TypeScript (`strict: true` in tsconfig.json)
- No unused locals or parameters (`noUnusedLocals`, `noUnusedParameters`)
- Use `type` imports for type‑only imports:
  ```ts
  import type { Person } from "~/lib/endpoint/types";
  import { createSignal } from "solid‑js";
  ```
- Prefer `async`/`await` over raw promises
- Use `Uint8Array` for binary data
- Use `bigint` for large integers (matching Rust `i64`/`u64`)
- Use `snake_case` for method names and properties (matching Rust IPC)
- Class methods that return promises should be marked `async`
- Use SolidJS reactive primitives (`createSignal`, `createEffect`, `createMemo`)

### Rust

- Edition 2024
- Use `eyre::Result` for internal error handling
- Convert to `Result<T, String>` for IPC via the `.mse()` extension
- Use `serde_json::Value` for JSON values crossing the IPC boundary
- Derive `Serialize`/`Deserialize` with `serde` for IPC types
- Use `taurpc` procedural macros for defining IPC endpoints
- Use `#[derive(Default)]` where appropriate
- Use `Arc<Slab<...>>` for handle‑based resource pools
- Use `tokio` for async runtime

### Imports Organization

1. Standard library / external crates
2. Internal modules
3. Type‑only imports
4. Relative imports

Example:

```rust
use std::sync::Arc;
use eyre::eyre;
use tauri::{Runtime, Window};
use crate::error::MapStringError;
```

### Error Handling

- **Rust**: Use `eyre::Result` and `eyre!` macro for internal errors. Convert to `String` with `.mse()` when returning over IPC.
- **TypeScript**: Use `try`/`catch` with `async` functions. Throw `Error` objects.
- **Playwright tests**: Use `expect` assertions; failures are reported as test failures.

### Project Structure

```
pupu/
├── Cargo.toml (workspace)
├── package.json
├── src/ (TypeScript frontend – SolidJS)
├── native/ (Tauri backend)
├── wasm/endpoint/ (Rust WebAssembly)
├── crates/ (shared Rust libraries)
├── cli/ (standalone relay binary)
├── tests/ (Playwright end‑to‑end tests)
├── public/ (static assets)
├── scripts/ (Python utilities)
└── docs/ (project documentation)
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `fix:` for bug fixes
- `feat:` for new features
- `docs:` for documentation changes
- `chore:` for maintenance tasks

### Key Technologies

- **Database**: Prisma for schema definition, Kysely for type‑safe SQL queries, SQLite via `tokio‑rusqlite`
- **IPC**: Tauri‑RPC with `#[taurpc::procedures]` traits, `#[taurpc::resolvers]` implementations, TypeScript bindings via `export_config`
- **WebAssembly**: Built with `wasm‑pack`, imported as `@pupu/endpoint`, functions exposed via `#[wasm_bindgen]`
- **Android**: Keystore in `keystore.properties`, build with `tauri android build`, icons from `public/icon.svg`

### Editor / Tooling

- No Cursor rules (`.cursorrules`) or Copilot instructions (`.github/copilot-instructions.md`) are present.

## Quick Reference

| Task             | Command                                   |
| ---------------- | ----------------------------------------- |
| Type check       | `bun run check`                           |
| Run all tests    | `bun run test` & `cargo test --workspace` |
| Lint Rust        | `cargo clippy --workspace`                |
| Format Rust      | `cargo fmt`                               |
| Start web dev    | `bun run dev`                             |
| Start native dev | `bun run native:dev`                      |
| Build web        | `bun run build`                           |
| Build native     | `bun run native:build`                    |
| Build Android    | `bun run android:build`                   |
