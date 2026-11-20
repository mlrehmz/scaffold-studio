# One-word commands. Needs: Rust (cargo) and Bun.
.PHONY: setup build test server web t specs watch fmt

setup:            ## install JS deps
	cd app/server && bun install
	cd app/web && bun install

build:            ## build the Rust `studio` binary the server calls
	cargo build --manifest-path core/Cargo.toml

test: build       ## everything that must be green before a commit
	cargo test --manifest-path core/Cargo.toml
	cd app/server && bun test && bunx tsc --noEmit
	cd app/web && bun test && bunx tsc --noEmit

server: build     ## terminal 1: http://localhost:3001
	cd app/server && bun run src/index.ts

web:              ## terminal 2: http://localhost:5173
	cd app/web && bunx vite

t:                ## ONE test while you iterate: make t T=notes::   (fast: only studio-core)
	cargo test --manifest-path core/Cargo.toml -p studio-core $(T) -- --include-ignored

specs:            ## the to-do list as red tests: every ignored spec and whether it passes yet
	@cargo test --manifest-path core/Cargo.toml --no-fail-fast -- --ignored 2>&1 | grep -E "^test .* \.\.\. " | sort

watch:            ## re-run one test on every save (needs: cargo install bacon)   make watch T=notes::
	cd core && bacon test -- -p studio-core $(T) -- --include-ignored

fmt:              ## format before every commit
	cargo fmt --manifest-path core/Cargo.toml --all
