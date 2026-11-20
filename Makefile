# One-word commands. Needs: Rust (cargo) and Bun.
.PHONY: setup build test server local-server web t specs watch fmt local-ai-setup local-ai-model local-ai-speech-model model model-lfm3

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

local-server: build ## run the app with the local text model proposer
	cd app/server && STUDIO_PROPOSER=local STUDIO_MODEL_URL=http://127.0.0.1:9931/v1/chat/completions bun run src/index.ts

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

local-ai-setup:   ## create the local CPU speech environment
	uv python install 3.12
	uv venv --python 3.12 app/local-ai/.venv
	uv pip install --python app/local-ai/.venv/bin/python -r app/local-ai/requirements.txt
	uv pip install --python app/local-ai/.venv/bin/python torch==2.14.0 --index-url https://download.pytorch.org/whl/cpu

local-ai-model:   ## download the recommended LFM3 GGUF model into the user cache
	mkdir -p $(HOME)/.local/share/scaffold-studio/models
	app/local-ai/.venv/bin/hf download LiquidAI/LFM3-1.7B-GGUF LFM3-1.7B-Q4_K_M.gguf --local-dir $(HOME)/.local/share/scaffold-studio/models

local-ai-speech-model: ## cache Whisper Small for local CPU transcription
	HF_HUB_DISABLE_XET=1 app/local-ai/.venv/bin/hf download Systran/faster-whisper-small model.bin config.json tokenizer.json vocabulary.txt

model:            ## run the local NanoI1 CPU model
	app/local-ai/.venv/bin/python app/local-ai/nano_server.py

model-lfm3:       ## run llama.cpp for the installed LFM3 model
	LD_LIBRARY_PATH=$(HOME)/.local/lib64 $(HOME)/.local/bin/llama-server -m $(HOME)/.local/share/scaffold-studio/models/LFM3-1.7B-Q4_K_M.gguf --alias LFM3-1.7B -c 4096 -t 4 -np 1 --host 127.0.0.1 --port 9931 --cors-origins http://localhost:5173,http://127.0.0.1:5173
