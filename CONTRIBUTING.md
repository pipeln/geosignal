# Contributing to GeoSignal

Thanks for helping improve GeoSignal. Please open an issue before a large change so the approach can be discussed.

## Principles

- Keep the default local experience local-first, loopback-only and safe by default.
- Do not add telemetry, accounts, hosted API keys or hidden network calls.
- Preserve the plain-language reports, fix guides and documented rulebook.
- Treat model weights and third-party content as separately licensed data.
- Do not weaken security checks, hash verification or public-mode restrictions to make a test pass.

## Development setup

You need Rust stable, CMake and a C/C++ compiler. On macOS:

```bash
xcode-select --install
brew install cmake
rustup toolchain install stable
```

Run the server (without desktop GTK/WebKit dependencies):

```bash
cargo run --release --no-default-features --bin geosignal
```

Run the desktop app:

```bash
./scripts/package-macos.sh
open "dist/GeoSignal Local SEO.app"
```

## Checks before a pull request

```bash
# Library and unit tests. The desktop binary needs macOS/WebKit and is not
# built by this CI command.
cargo test --locked --lib --no-default-features

# Browser/server binary without desktop GTK/WebKit dependencies
cargo build --release --locked --bin geosignal --no-default-features

# Container build (use linux/amd64 for Hugging Face Spaces)
docker build --platform linux/amd64 -t geosignal .
```

For a local UI change, run the server and open <http://127.0.0.1:4173/>. For a desktop-only change, run the packaged app and the UI smoke test:

```bash
GEOSIGNAL_DATA="$PWD/data" \
  "./dist/GeoSignal Local SEO.app/Contents/MacOS/GeoSignal" --ui-smoke
```

## Public/Docker changes

The Docker image intentionally uses `GEOSIGNAL_PUBLIC=1`. Any change to bind addresses, `Host`/`Origin` checks, crawler URL validation or model/update URL validation must be reviewed as a security change. Add tests for both the local default and the public mode.

Do not commit:

- `data/`, model weights, partial downloads or scan results;
- `.env` files or API keys;
- local screenshots that contain machine paths or customer data;
- generated `target/` or `dist/` output.

## Pull requests

- Keep changes focused and explain the user-visible reason.
- Include tests for behavior changes, especially security and download verification.
- Update the relevant Markdown documentation when behavior, configuration or deployment changes.
- Mention which checks you ran and any checks you could not run.

## Licensing

By contributing, you agree that your contribution is licensed under the repository's MIT license. Do not add third-party code, model weights or copied documentation without compatible terms and attribution. See `LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.md` and `MODEL_LICENSES.md`.
