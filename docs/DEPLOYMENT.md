# Docker and Hugging Face Spaces

GeoSignal's Docker image runs the same local server and UI as the terminal version. The difference is configuration: the image deliberately binds `0.0.0.0:7860` and enables `GEOSIGNAL_PUBLIC=1` so a container platform can route traffic to it.

## Important: Hugging Face plan requirement

Hugging Face currently documents that **Docker Spaces require a paid plan** (PRO for a personal account, or Team/Enterprise for an organization). The **CPU Basic** hardware itself has no hourly charge, but creating a Docker Space is not available on the free plan. Free accounts can host static Spaces, and a limited number of Gradio ZeroGPU Spaces, but not Docker Spaces.

Use Docker on any other host if you do not have a Hugging Face paid plan. The same image runs on a local Docker engine, a VPS, Fly.io, Railway, Render, or another container host. Check that host's terms and resource limits before exposing a public instance.

## Build and run locally

```bash
mkdir -p data
# On Linux only: sudo chown -R 1000:1000 data
docker build --platform linux/amd64 -t geosignal .
docker run --rm -p 127.0.0.1:7860:7860 -v "$PWD/data:/data" geosignal
```

Open <http://127.0.0.1:7860/>.

The image uses `linux/amd64`, which is the architecture used by Hugging Face Spaces. On Apple silicon, Docker Desktop uses emulation for an amd64 build. You can build a native image for local-only testing with `docker build -t geosignal .`, but publish the amd64 image for Spaces.

Or use Compose, which publishes only to host loopback by default:

```bash
docker compose up --build
```

On Linux, make the bind mount writable by the image user (UID 1000) before starting the container. On Docker Desktop for macOS, file sharing normally handles this.

## Hugging Face Space setup

1. Create a new Space and choose **Docker** as the SDK.
2. Put this repository in the Space repository. The root `README.md` already contains:

   ```yaml
   ---
   title: GeoSignal Local SEO
   emoji: 📍
   colorFrom: green
   colorTo: blue
   sdk: docker
   app_port: 7860
   ---
   ```

3. Commit the root `Dockerfile`, `docker/entrypoint.sh`, `public/`, `src/`, `Cargo.toml` and `Cargo.lock` with the rest of the repository.
4. The image starts the server on port `7860`. No Space variable is required for a basic run.
5. Wait for the build. The first build compiles llama.cpp and the Rust dependency graph and can take several minutes.
6. Open the Space URL and run a public site scan.

### Storage

Without a Storage Bucket, `/data` is ephemeral and the Space loses scan history and downloaded models when it rebuilds. Attach a Storage Bucket in **Space settings → Storage Buckets**, mount it at `/data`, and recreate/restart the Space. The entrypoint uses `/data` automatically when it exists and is writable. If `/data` does not exist, it uses `/home/user/data`. If `/data` exists but is not writable, startup fails with an explicit error rather than silently switching to ephemeral storage.

A bucket is strongly recommended if you use the Model lab. The built-in models are 1–2 GB and will be downloaded again after an ephemeral restart.

### Resource notes

- CPU Basic: 2 vCPU, 16 GB RAM, 50 GB non-persistent disk. The small built-in models run on CPU, but generation is slower than on a Mac with Metal.
- The Server is unauthenticated. A public Space exposes its scan history to visitors.
- Public mode caps a crawl at 10 pages, blocks private/loopback/link-local hosts and re-checks the resolved address at connect time, so the Space cannot be used as an internal network proxy.
- At most three audits run at once; further requests get HTTP 429 rather than queueing.
- Model downloads and crawls use the Space's network, CPU, memory and disk. Do not invite abusive traffic.

### Space variables

The image sets these defaults:

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `7860` | Container listen port |
| `GEOSIGNAL_BIND` | `0.0.0.0` | Bind address inside the container |
| `GEOSIGNAL_PUBLIC` | `1` | Public host/origin checks and SSRF limits |
| `GEOSIGNAL_DATA` | selected by entrypoint | Scan/model data directory |

Optional variables:

- `GEOSIGNAL_ASSETS` — path to the built UI assets.
- `GEOSIGNAL_UPDATE_MANIFEST` — default HTTPS update manifest.
- `OLLAMA_HOST` — optional Ollama endpoint. Do not point this at an untrusted host; prompts and crawl text are sent there.

Do not put private API keys in the image or repository. GeoSignal does not need one for its built-in models.

## Restricting a container to this machine

The image defaults to public mode because its intended target is a public Space. Mapping the published port to `127.0.0.1` keeps the instance off your LAN while leaving the container's public-mode checks in place:

```bash
docker run --rm -p 127.0.0.1:7860:7860 \
  -v "$PWD/data:/data" \
  geosignal
```

The server inside the container must still bind `0.0.0.0`, because Docker forwards published ports to the container interface, not to its loopback. Public mode therefore remains on. For private or staging sites that public mode refuses, use the native app or a local server with `GEOSIGNAL_PUBLIC` unset; do not disable the guard just to scan an internal address from a public-style container.

To expose the container to your LAN deliberately, publish `0.0.0.0:7860:7860` and understand that there is no authentication. Do not do this on an untrusted network.

## Updating a Space

The app's **Updates** tab changes help content and model definitions inside the running container's data directory. That is separate from deploying a new image.

To ship code changes:

1. Commit and push the new source to the Space repository.
2. Let the Space rebuild.
3. If the update changes environment settings, update the Space variables, not the image.
4. Re-check the Space logs if the health page does not load.

Content updates written under `/data` survive a rebuild only when a persistent bucket is attached.

## Troubleshooting

- **Build fails in the Rust/llama.cpp stage** — ensure the Docker builder has at least 4 GB of memory and enough disk. The image installs `cmake`, `clang` and `libclang-dev` in the builder stage.
- **Container starts but the page does not load** — check that the Space is using port `7860` and that the container is listening on `0.0.0.0`. `GEOSIGNAL_BIND` must not be left at `127.0.0.1` for a Space.
- **Permission denied under `/data`** — on Linux, `chown -R 1000:1000 ./data`; on Spaces, attach a writable bucket at `/data`. If the container exits with `/data exists but is not writable`, fix the mount ownership or set `GEOSIGNAL_DATA` to a writable directory.
- **A scan refuses a private address** — that is public mode working as intended. Use the native app or a loopback server for internal or staging sites.
- **Model downloads disappear after a rebuild** — attach a bucket and mount it at `/data`.
- **Hugging Face says Docker Spaces need a paid plan** — that is a platform plan restriction, not a GeoSignal error. Use another container host or upgrade the account.

## GitHub

The repository includes a CI workflow that runs the library tests and builds the server binary and Docker image:

```bash
cargo test --locked --lib --no-default-features
cargo build --release --locked --bin geosignal --no-default-features
docker build --platform linux/amd64 -t geosignal:ci .
```

No registry push is configured. Add a publish workflow only after choosing a registry and credentials.
