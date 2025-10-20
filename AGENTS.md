# Bootstrap Project Guide

## Project Overview
Bootstrap is an experimental compiler toolchain for a Rust-inspired systems language that targets both WebAssembly (Wasm) and WebGPU's WGSL. The repository contains:

- **Stage1 compiler sources** in `.bp` (Bootstrap language) that can compile themselves.
- **Stage2 compiler artifact** (`compiler.wasm`) generated from the Stage1 sources.
- **TypeScript host runtime** that embeds the Stage2 compiler, drives compilation, and exposes a CLI built for Bun.
- **Extensive Bun test suite** validating language semantics, parser behaviour, and host integration.
- **Design documentation** describing compilation pipeline, type templates, and open proposals.

## Directory Layout
- `src/`: TypeScript entrypoints shared by the CLI and library consumers. Uses ES modules, modern TypeScript syntax, and Bun APIs for file I/O.
- `compiler/`: Bootstrap language modules (`.bp`) for the Stage1 compiler. `ast_compiler.bp` is the entry module used when rebuilding the Stage2 Wasm.
- `stdlib/`: Core intrinsic modules consumed by compiled programs (`memory.bp` currently ships with the Stage2 runtime).
- `test/`: Bun-powered unit tests exercising the compiler and runtime interfaces.
- `examples/`: Sample `.bp` programs.
- `docs/`: Markdown notes and static site assets explaining the architecture and research direction.
- `compiler.wasm`: Prebuilt Stage2 compiler used by the TypeScript host when compiling user code.

## General Contribution Guidelines
- Prefer Bun's toolchain. Use `bun test` before submitting changes that affect TypeScript, compiler code, or runtime behaviour.
- You can rebuild the Stage2 compiler after modifying `.bp` files under `compiler/` or `stdlib/` by running `bun ./src/cli.ts` (with no arguments). This regenerates `compiler.wasm`. Avoid doing this except when necessary.
- Keep documentation in `docs/` up to date when altering the compilation pipeline or language semantics.
- When adding tests, follow the existing structure in `test/`, using descriptive filenames and `describe`/`test` blocks.

## TypeScript Style Notes (`src/`, `test/`)
- Use double quotes for strings and prefer `const`/`readonly` where possible.
- Propagate errors through the existing `CompileError` class rather than throwing plain strings.
- Maintain explicit return types on exported functions and keep Bun-specific behaviour behind feature checks.
- Keep the CLI user messages concise and mirror the existing phrasing conventions.

## Bootstrap Language Style Notes (`.bp` files)
- Indent with four spaces per level and terminate statements with semicolons, matching the current modules.
- Group `use` declarations at the top of each module and prefer relative module paths.
- Follow existing naming conventions (`snake_case` for functions and locals, `CamelCase` for types when added).
- Keep memory offsets and layout helpers in `utils.bp`/`memory.bp`; avoid duplicating low-level constants in new modules.
- Document non-trivial control flow or memory operations with `//` comments.

Adhere to these guidelines for any files within this repository. Nested directories may introduce additional instructions in their own `AGENTS.md` files; check before editing.
