---
title: GeoSignal Local SEO
emoji: 📍
colorFrom: green
colorTo: blue
sdk: docker
app_port: 7860
---

# GeoSignal Local SEO

GeoSignal is a local-first **local SEO auditor** for small businesses and the people who help them. It crawls a public website, scores the local-search signals Google documents, and explains every finding in plain language — with evidence, fix steps, copyable examples, and links to the source guidance.

GeoSignal runs in three ways: as a native **macOS app**, as a **local browser/server** process, or as a **Docker image** for a container host such as Hugging Face Spaces. All three use the same rulebook, reports, Model lab and Updates UI. There is **no account, no telemetry, no hosted API, and no API key**.

> GeoSignal estimates local-search readiness. It does **not** report Google rankings, impressions, clicks, or Business Profile performance.

## What is included

- **Quick check** — crawl a homepage plus internal pages, then score **34 documented checks** across five areas: local entity, on-page relevance, location linking, technical access, and trust.
- **Plain-language report** — see the score, category breakdown, priority findings, evidence for every result, and what is already working.
- **How-to-fix guides** — step-by-step help for findings, including copyable examples and who can help. Help topics are written for business owners, not developers.
- **Tips toggle** — turn the `?` help buttons off when you already know the interface.
- **Model lab** — optionally download one of three small GGUF models and run an AI cross-check entirely on the machine running GeoSignal. No Ollama is required for the built-in models.
- **Custom models** — add any GGUF model that is not in the built-in catalog with an HTTPS URL and a pinned SHA-256. The model stays local and is never loaded until its download verifies.
- **Model comparison** — compare up to six local models, see JSON validity, latency, schema compliance, and agreement, and separate consensus findings from one-model opinions.
- **Updates** — point the app at an HTTPS manifest to refresh help content and model-catalog entries without rebuilding, and to check for a newer app release.
- **Standards** — inspect the rulebook, the Google source behind each check, and local overrides for weights and custom rules.
- **Local reports** — open the full HTML report, download Markdown, or return to any recent scan from the local history.

## Requirements

### Native macOS app

- macOS 11 or newer
- Xcode Command Line Tools (`xcode-select --install`)
- Rust stable via [rustup](https://rustup.rs/)
- CMake and a C/C++ compiler for the embedded llama.cpp build

### Browser/server mode

- Rust stable and the same C/C++ build requirements
- A modern browser

### Docker mode

- Docker Desktop or a Docker Engine
- A container host, or [Hugging Face Spaces](https://huggingface.co/docs/hub/spaces-sdks-docker) with a paid plan for a Docker Space

The repository does not contain model weights. The image downloads them on demand at runtime, not at build time.

## Quick start

### Run the desktop app

```bash
./scripts/package-macos.sh
open "dist/GeoSignal Local SEO.app"
```

The app starts its own loopback-only server and shows it in a native window. The bundle is **ad-hoc signed, not notarized**, so macOS may ask you to confirm the first launch. Right-click the app and choose **Open** if needed.

### Run the browser version

```bash
cargo run --release --no-default-features --bin geosignal
```

Open <http://127.0.0.1:4173/>. Choose another port if needed:

```bash
PORT=8080 cargo run --release --no-default-features --bin geosignal
```

The local server has no authentication. Keep it on `127.0.0.1`; do not expose it through a reverse proxy, tunnel, or port forwarding unless you intentionally use the Docker public mode described below.

### Run the Docker image

```bash
mkdir -p data
# On Linux only: sudo chown -R 1000:1000 data
docker build --platform linux/amd64 -t geosignal .
docker run --rm -p 127.0.0.1:7860:7860 -v "$PWD/data:/data" geosignal
```

Open <http://127.0.0.1:7860/>. The example publishes only on the host loopback; the container itself listens on `0.0.0.0:7860` and uses public mode, so browser host/origin checks and public crawler limits are active. Do not replace `127.0.0.1` with `0.0.0.0` in the port mapping unless you intend to expose the unauthenticated instance to your network. For a local Compose run:

```bash
docker compose up --build
```

The container runs as user `1000`. If the host data directory is not writable by that user, fix its ownership before starting it; the entrypoint will stop with an explicit error rather than silently using ephemeral storage.

Hugging Face Spaces needs the `sdk: docker` metadata already present at the top of this README, plus the Dockerfile at the repository root. See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for the Space setup, storage, memory and plan details.

## Using GeoSignal

1. Enter a website address. Optionally add the primary service and city or service area.
2. Choose how many pages to crawl (1–30) and run the check.
3. Read the report in this order:
   - **Score and category breakdown** — where the site is strong or weak.
   - **Findings** — the highest-impact problems, with evidence and a fix.
   - **How to fix this** — plain-language steps and copyable code.
   - **Tested checks** — what passed and why it matters.
   - **Pages crawled** — the exact URLs and crawl status.
4. Optionally open **Model lab**, download a model, and run a cross-check from the report.
5. Optionally open **Updates** to refresh help content and check for a newer release.
6. Export the **Full report** (HTML) or **Markdown** when you need to share it.

The **Tips on/off** control in the header hides the help buttons without hiding the fix guides.

### How the crawl works

GeoSignal fetches the pages you point it at, follows internal links up to two clicks deep, reads `robots.txt`, and looks for a sitemap. It sends a clear GeoSignal user agent. Pages are read-only; GeoSignal never edits the site.

In the native app and local server modes, a scan fetches the URL you enter, including a staging or private host if you type one. Docker public mode refuses private and loopback targets. Only scan sites you own or are authorised to audit.

## Models

The Model lab can download these quantised GGUF presets directly into the local data folder:

| Preset | Size | Upstream | License |
| --- | ---: | --- | --- |
| Qwen 2.5 1.5B | 1.1 GB | [Qwen GGUF](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF) | [Apache-2.0](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/blob/main/LICENSE) |
| Llama 3.2 3B | 2.0 GB | [bartowski GGUF](https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF) | [Meta Llama 3.2 Community License](https://llama.meta.com/llama3_2/license) |
| Gemma 2 2B | 1.7 GB | [bartowski GGUF](https://huggingface.co/bartowski/gemma-2-2b-it-GGUF) | [Gemma Terms of Use](https://ai.google.dev/gemma/terms) |

- Downloads resume after interruption and are verified against pinned SHA-256 digests before use.
- Model files are **not** committed to this repository and are **not** bundled in the app or Docker image.
- In a container, downloaded weights and scan history vanish when the container is replaced unless `/data` is persisted. Hugging Face Spaces needs a Storage Bucket for persistence.
- By downloading a model you agree to its upstream license. The app shows the license name; review the linked terms before downloading. See [MODEL_LICENSES.md](MODEL_LICENSES.md).
- AI suggestions are a cross-check, not the source of truth. The documented rulebook score is authoritative.
- **Ollama is optional.** Set `OLLAMA_HOST` (default `http://127.0.0.1:11434`) to use custom Ollama models. Anything sent to a non-local Ollama host leaves your machine.

### Custom GGUF models

Open **Model lab → Custom GGUF models** and add:

- a display name;
- an HTTPS download URL for the exact `.gguf` file;
- the file’s 64-character SHA-256 digest;
- a license name and optional family/notes.

The file is downloaded on demand, checked against the digest, and stored in the same local models folder as the presets. A successful download records the exact URL and SHA-256 used. If an entry keeps the same filename but changes either value — including through a content-pack catalog update — the old file is treated as unverified and must be downloaded again. A leftover partial download from the old URL or digest is discarded before the fresh attempt starts. A failed check deletes the partial file. Removing a custom entry also removes its downloaded file. The 20 GB limit is a safety bound, not a recommendation; 1–3B quantised models are the practical sweet spot for this task.

Content packs use the same registry: a model added by a content update appears in the Model lab and must still be downloaded explicitly.

## Updates

The **Updates** tab reads an optional HTTPS manifest. Updates are **data-only** and never execute remote code. The app can refresh:

- help topics and fix guides;
- additional model-catalog definitions;
- the latest app version, release notes and release-page link.

The app checks a release but does not replace its own signed macOS bundle. Installing a new app build stays a deliberate download and install from the release page.

Set the manifest URL in the Updates tab, or provide a default at build/run time:

```bash
GEOSIGNAL_UPDATE_MANIFEST="https://example.com/geosignal/manifest.json" \
  cargo run --release --no-default-features --bin geosignal
```

The saved source and installed content pack live in `data/updates/`. The URL must use HTTPS. A minimal manifest:

```json
{
  "schema": 1,
  "app": {
    "latest": "1.1.0",
    "releaseUrl": "https://github.com/you/geosignal/releases/tag/v1.1.0",
    "notes": "Adds a new comparison view."
  },
  "content": {
    "version": "2026.11.1",
    "url": "https://example.com/geosignal/content-2026.11.1.json",
    "sha256": "64 lowercase hex characters",
    "notes": "Refreshed title and schema guidance."
  }
}
```

`sha256` is the digest of the content file, not the manifest. A content pack can replace or add help topics and guides, and add model definitions:

```json
{
  "schema": 1,
  "version": "2026.11.1",
  "notes": "Refreshed title and schema guidance.",
  "topics": {
    "updates_source": {
      "title": "Update source",
      "body": "The HTTPS manifest GeoSignal reads when you press Check for updates."
    }
  },
  "guides": {},
  "models": [
    {
      "label": "My verified model",
      "url": "https://example.com/models/my-model.gguf",
      "file": "my-model.gguf",
      "sha256": "64 lowercase hex characters",
      "bytes": 1500000000,
      "license": "Apache-2.0"
    }
  ]
}
```

Publish both files over HTTPS and update the manifest hash whenever the content changes. The app verifies the content hash before writing anything; a mismatch or malformed pack leaves the existing content untouched. Use **Restore built-in help** in the Updates tab to remove an installed pack.

Scores, crawl logic, security rules and the app binary are not updateable content. Changing those requires a new signed app build.

## Data and privacy

GeoSignal stores data locally:

| Mode | Default data folder |
| --- | --- |
| Browser/server from the repository | `data/` in the current directory |
| Packaged app launched from the repository | `data/` in the project directory |
| Packaged app launched from Finder | `~/Library/Application Support/GeoSignal/data` |
| Docker, writable `/data` (Compose or an attached Space bucket) | `/data` |
| Docker without `/data` | `/home/user/data` (ephemeral) |

Set `GEOSIGNAL_DATA` to use another folder.

Stored files include scan history, report JSON, comparisons, overrides, downloaded models, the custom model catalog (`data/model-catalog.json`), and update settings/content (`data/updates/`). The data folder can contain **customer URLs, page text, and business contact details**. Treat it as private. `data/` is excluded from Git.

A public Docker or Hugging Face Space exposes its scan history and reports to every visitor. Use a separate private instance for client work.

GeoSignal does not upload scan data. The only outbound requests are:

- the website being crawled;
- the model host when you explicitly download a built-in or custom model;
- your configured Ollama host, if you enable custom Ollama models;
- your configured update manifest and content-pack host when you press Check or Install.

There is no analytics, crash reporting, account system, or remote logging.

## Security model

### Native app and local server

The default HTTP server is intentionally local:

- binds to **127.0.0.1 only**;
- has **no authentication** and must not be exposed;
- validates `Host` and browser `Origin` headers on state-changing requests;
- limits request bodies to 1 MB;
- rejects dotfiles, backslashes, and path traversal in static file serving;
- sends CSP and standard browser hardening headers.

### Docker and Hugging Face Spaces

The Docker image deliberately enables a public mode with `GEOSIGNAL_PUBLIC=1` and binds `0.0.0.0:7860`. This is an explicit opt-in: without `GEOSIGNAL_PUBLIC=1`, the server refuses a non-loopback `GEOSIGNAL_BIND`. Public mode has additional limits:

- browser `Origin` values must match the request `Host`, including any non-default port, for state-changing requests;
- the crawler rejects loopback, private, link-local, single-label and metadata hostnames, including redirects;
- the resolved address is checked again at connect time, so a hostname cannot pass the URL check and then resolve to a private address;
- crawls are capped at 10 pages per run instead of 30;
- at most three audits run at once; additional requests get HTTP 429 instead of queueing;
- custom model and update URLs must resolve to public hosts;
- update downloads do not follow redirects in public mode.

There is still **no authentication**. A public Space is a shared, public instance: anyone can run scans, see the local scan history and use model downloads. Do not put customer, private or confidential data in a public instance. Run the native app or a loopback server for private audits. Model downloads and scans consume the host’s bandwidth, memory and disk.

Model downloads use HTTPS and pinned SHA-256 hashes. A hash mismatch deletes the partial download instead of installing it. Custom model URLs, update manifests and content packs are also HTTPS-only; content packs must match their pinned SHA-256, and the update source cannot supply executable code.

To report a security issue, open a private security advisory on the repository rather than a public issue with exploit details.

## Build and test

```bash
# Unit tests (no GTK/WebKit desktop dependencies required)
cargo test --locked --lib --no-default-features

# Build the browser/server binary without desktop GTK/WebKit dependencies
cargo build --release --locked --bin geosignal --no-default-features

# Build the container
docker build --platform linux/amd64 -t geosignal .

# Build the native app and copy it to dist/
./scripts/package-macos.sh

# Packaged server lifecycle smoke test
GEOSIGNAL_DATA="$PWD/data" \
  "./dist/GeoSignal Local SEO.app/Contents/MacOS/GeoSignal" --ui-smoke

# Dependency advisory scan (install cargo-audit first)
cargo install cargo-audit --locked
cargo audit
```

The UI smoke test opens the real window, stops and restarts the in-process server, opens a fix guide, and verifies that an external source link reaches the native URL opener.

## Project layout

| Path | Purpose |
| --- | --- |
| `src/bin/desktop.rs` | Native macOS window, server controls, IPC, and UI smoke test |
| `src/server.rs` | Loopback HTTP server, API routes, static files, and security checks |
| `src/scan.rs` | Crawler, robots.txt, sitemap, and page fetching |
| `src/html.rs` | HTML parsing and link/image extraction |
| `src/schema.rs` | JSON-LD and LocalBusiness/N.A.P. extraction |
| `src/rules.rs` | The 34-check rulebook and scoring |
| `src/report.rs` | HTML and Markdown report generation |
| `src/ai.rs` | Model presets, prompts, comparison, and curation |
| `src/models.rs` | Custom GGUF registry, validation, and local catalog storage |
| `src/engine.rs` | Embedded llama.cpp inference and verified model downloads |
| `src/updates.rs` | HTTPS update manifests, verified content packs, and release checks |
| `src/store.rs` | Local scan, override, and comparison storage |
| `public/` | Browser UI, styles, and plain-language `guides.js` |
| `scripts/package-macos.sh` | Build and package `GeoSignal Local SEO.app` |
| `Dockerfile` | Multi-stage Linux image for Docker and Hugging Face Spaces |
| `docker/entrypoint.sh` | Chooses a writable data directory and starts the server |
| `docker-compose.yml` | Local container run with a persistent `./data` mount |
| `docs/DEPLOYMENT.md` | Docker and Hugging Face Spaces deployment guide |

## Troubleshooting

**`cargo` or `cmake` not found** — install Rust with rustup and CMake (for example `brew install cmake`), then reopen the terminal.

**The app does not open** — it is ad-hoc signed and not notarized. Right-click → **Open**, or build it locally with the packaging script.

**A model download fails** — check free disk space and network access to the model host. Interrupted downloads resume; a hash mismatch removes the partial file and can be retried. For a custom model, confirm the SHA-256 is for the exact file behind the URL.

**No model appears in Model lab** — the rulebook works without a model. Open Model lab and download a preset, add a custom GGUF model, or start Ollama and set `OLLAMA_HOST` if you want to use an Ollama tag.

**An update download fails** — confirm the manifest and content URLs are HTTPS, the manifest uses `"schema": 1`, and the content `sha256` matches the file you published. The Updates tab shows the last error; **Restore built-in help** removes an installed pack.

**Port 4173 is busy** — use `PORT=8080 cargo run --release --no-default-features --bin geosignal`. The desktop app chooses an ephemeral loopback port automatically.

**Docker build is slow or runs out of memory** — the first build compiles llama.cpp and the Rust dependency graph. Use a machine with at least 4 GB of free memory and a Docker builder with enough disk. Subsequent builds reuse layers.

**The Docker container cannot write data** — the image runs as user `1000`. On Linux, `sudo chown -R 1000:1000 ./data` before starting Compose. If `/data` exists but is not writable, the entrypoint stops with an error instead of silently using ephemeral storage. On Hugging Face Spaces, attach a Storage Bucket at `/data`; without one, the container uses `/home/user/data` and data is ephemeral.

**A Docker Space shows “private address”** — public mode intentionally refuses loopback, private, link-local and metadata hosts. Scan a publicly reachable site instead.

**A link does not open from the desktop app** — GeoSignal only opens `http`, `https`, `mailto`, and `tel` links. Other schemes are blocked deliberately.

## Contributing and security

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and pull-request checks. Report vulnerabilities privately using the process in [SECURITY.md](SECURITY.md), not in a public issue.

## Licenses

- **GeoSignal source code:** MIT — see [LICENSE](LICENSE).
- **Attribution notice:** see [NOTICE](NOTICE).
- **Rust crates linked into the binaries:** full attribution and license texts are in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
- **Embedded inference engine:** llama.cpp / ggml, MIT. Its notice is included in the third-party file.
- **Model weights:** separate upstream terms, listed in [MODEL_LICENSES.md](MODEL_LICENSES.md). No weights are included in this repository.

The app bundle includes `LICENSE`, `NOTICE`, `THIRD_PARTY_LICENSES.md`, `MODEL_LICENSES.md`, and `licenses/` in `Contents/Resources`. The Docker image carries the same files in its app directory. Custom models and content packs are data files, not part of the app or image; their upstream terms remain the publisher’s.

Third-party notices can be regenerated from the locked dependency graph with:

```bash
cargo about generate --format json --locked -o /tmp/geosignal-licenses.json
node scripts/generate-third-party-licenses.mjs /tmp/geosignal-licenses.json
```

`cargo about` can be installed with `cargo install cargo-about --locked --features cli`.

## Disclaimer

GeoSignal is not affiliated with, endorsed by, or sponsored by Google, Meta, Alibaba, or Google DeepMind. Google, Google Business Profile, Search Console, Meta Llama, Qwen, and Gemma are trademarks of their respective owners.

Scores are estimates based on a live crawl of documented signals. They are not a guarantee of rankings, traffic, or business results.
