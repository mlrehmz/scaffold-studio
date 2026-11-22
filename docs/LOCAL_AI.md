# Local AI

Scaffold Studio can run text generation, speech recognition, and embeddings locally. Local model output is treated as a draft; the Rust core remains responsible for validating design changes.

## Text models

NanoI1 is the default local text proposer and runs on CPU. LFM3 is available as an alternate. Model files are downloaded by the setup commands and kept on the local machine.

```sh
make local-ai-setup
make model
```

Select the rules based proposer with `STUDIO_PROPOSER=rules`. Start the alternate LFM3 runtime with `make model-lfm3`.

## Speech and embeddings

The application includes local speech recognition and browser based embedding support. Model setup and execution happen on the user's device; first use may download model files.

## Runtime notes

Small local models can return malformed or incomplete output. The application handles invalid proposals through its normal validation and fallback paths. CPU speed and output quality depend on the device and model.
