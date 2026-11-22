# Scaffold Studio

Scaffold Studio is a visual workbench for describing software structure and preparing implementation guidance. People review proposed changes before applying them, and the Rust core validates design changes.

## Features

- Create and review components, relationships, constraints, and implementation steps.
- Inspect proposed changes before applying them.
- Keep local conversation history and preferences for project work.
- Generate TypeScript scaffolds and check them against a design.
- Explore process guidance, patterns, drift, and sketch based proposals.
- Run local text, speech, and embedding models; see [Local AI](docs/LOCAL_AI.md).

## Architecture

- `core/`: Rust domain model, validation, and design rules.
- `app/server/`: Bun server and application services.
- `app/web/`: React interface.
- `.studio/`: local project data, excluded from Git.

The Rust core is authoritative. Model output remains a proposal until reviewed and validated.

## Run locally

Requirements: Rust and Cargo, Bun, Make, and `uv` for local model setup.

```sh
make setup
make test
make local-ai-setup
```

Start the local model and application in separate terminals:

```sh
make model
make server
make web
```

Open the web app at `http://localhost:5173`. The first model run downloads the pinned model. To use the rules based proposer, set `STUDIO_PROPOSER=rules`. To use the alternate LFM3 model, run `make model-lfm3`.

## Local data

Project data and downloaded models are stored locally. Project data under `.studio/` is not added to Git.
