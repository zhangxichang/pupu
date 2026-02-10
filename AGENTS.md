# AGENTS.md

This document provides guidelines for AI agents working on this codebase.

## Project Overview

Pupu is a SolidJS/SolidStart application with both web (Cloudflare Workers) and native (Tauri) targets. It uses TypeScript, TailwindCSS v4, Rust/WASM for cryptographic endpoints, and SQLite for data persistence.

## Build, Lint, and Test Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `bun run dev` | Start development server (web version) |
| `bun run build` | Build web version with type checking and linting |
| `bun run check` | Run TypeScript compiler and ESLint |
| `bun run test` | Run Playwright tests |
| `bun run preview` | Preview the built web version (port 8787) |

### Native (Tauri) Commands

| Command | Description |
|---------|-------------|
| `bun run dev:native` | Start Tauri native development server |
| `bun run build:native` | Build Tauri native application |
| `bun run android:init` | Initialize Android build environment |
| `bun run android:build` | Build Android APK (aarch64) |

### Running a Single Test

Run a specific test file:
```bash
bun playwright test tests/auth.test.ts
```

Run a specific test by name:
```bash
bun playwright test -g "登录"
```

Run tests with a specific project:
```bash
bun playwright test --project=chromium
```

### Code Generation

| Command | Description |
|---------|-------------|
| `bun run generate:db-schema` | Generate Prisma schema and push to database |
| `bun run generate:icon` | Generate Tauri app icons from SVG |

### Rust/WASM Commands

The Rust endpoint is built via `wasm-pack` during `install:pre`:
```bash
wasm-pack build wasm/endpoint --target web --release
```

## Code Style Guidelines

### TypeScript Configuration

- **Strict Mode**: Enabled in `tsconfig.json`
- **Path Alias**: Use `~/*` for imports from `src/*` directory
- **No Unused**: Both `noUnusedLocals` and `noUnusedParameters` are enabled

### ESLint Rules

The project extends:
- ESLint recommended rules
- TypeScript-ESLint recommended type-checked rules

Strict rules enabled:
- `@typescript-eslint/strict-boolean-expressions` (error)
- `@typescript-eslint/no-misused-promises` (disabled, not strict in this codebase)

### Formatting and Imports

- **No Prettier config**: Project uses minimal formatting (empty `.prettierrc`)
- **ESLint handles formatting**: No `prettier/prettier` rule
- **Import syntax**: Use TypeScript ESM imports with `.js` extension for relative imports
- **Absolute imports**: Use `~/*` alias for source files

### Naming Conventions

| Pattern | Convention | Example |
|---------|------------|---------|
| Classes | PascalCase | `HomeStore`, `QueryBuilder` |
| Interfaces | PascalCase | `Store`, `Init`, `Free` |
| Variables/Constants | camelCase | `sqlite`, `userId` |
| Functions | camelCase | `init()`, `cleanup()` |
| Private members | Prefix with `_` or use `private` keyword | `private constructor` |
| Types | PascalCase | `Person`, `EndpointConfig` |
| Files | kebab-case for general files | `query-builder.ts` |

### Error Handling

- Use `throw new Error("message")` for runtime errors
- Provide descriptive error messages (Chinese messages are acceptable since UI is in Chinese)
- Consider using `Result` pattern for functions that may fail

### TypeScript Best Practices

- **Explicit types**: Do not rely on type inference for public APIs
- **No `any`**: Avoid `any` type; use `unknown` if necessary
- **Strict null checks**: Handle `null` and `undefined` explicitly
- **Use `interface` vs `type`**: Use `interface` for object shapes, `type` for unions/primitives
- **Async/await**: Prefer `async/await` over promise chains
- **Event types**: Define event handler types explicitly

### SolidJS Patterns

- **Store pattern**: Use class-based stores implementing `Store` interface
- **Signals**: Use `createSignal`, `createStore` for reactive state
- **Components**: Use `.tsx` files for components with `props` interface
- **Resources**: Use `createResource` for async data fetching

### Database (Kysely + SQLite)

- **Query Builder**: Use `QueryBuilder` for type-safe SQL queries
- **Generated types**: Prisma/Kysely generates types from schema
- **Raw queries**: Use `sqlite.query()` with compiled QueryBuilder
- **Schema**: Located in `prisma.config.ts`

### Rust Integration

- **Endpoint**: WASM module in `wasm/endpoint/`
- **Native**: Rust library in `native/src/`
- **IPC**: Uses Tauri's IPC for native communication
- **Build**: Rust is compiled during dependency installation

### CSS and Styling

- **TailwindCSS v4**: Use utility classes from Tailwind
- **DaisyUI**: Component library (version 5.x)
- **Tailwind Merge**: Use `twMerge` for class merging
- **Theme Change**: Use `theme-change` for dark/light mode

### File Organization

```
src/
├── app.config.ts         # App configuration
├── lib/
│   ├── sqlite/          # SQLite adapters (web/native)
│   ├── endpoint/        # WASM endpoint wrappers
│   ├── types.ts         # Shared types
│   └── sqlite.ts        # SQLite factory
├── stores/
│   ├── main.ts          # Main application store
│   ├── home.ts          # Home screen store
│   └── interface.ts     # Store interfaces
└── routes/              # File-based routing
```

### Commit Guidelines

- Write commit messages in English or Chinese
- Reference issue numbers when applicable
- Use conventional commits format for features

### Additional Notes

- **Node version**: Uses ESNext features
- **Module type**: ESM (`"type": "module"`)
- **Package manager**: Bun is the primary package manager
- **Testing framework**: Playwright for E2E tests
- **Database**: SQLite with Kysely query builder
