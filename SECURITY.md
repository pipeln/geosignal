# Security policy

## Reporting a vulnerability

Please do not open a public issue with exploit details. Use GitHub's **Report a vulnerability** / private security advisory feature for this repository, or contact the maintainer privately through the repository profile.

Include the affected version, deployment mode (native app, local server or Docker/public), reproduction steps and the impact you believe is possible. Do not include real customer data, credentials or model files in a report.

## Supported versions

Security fixes are applied to the current `1.x` line. Older builds may not receive backports.

## Deployment modes

### Native app and local server

The default server binds only to `127.0.0.1`. It has no authentication and is intended for one user on one machine. Do not expose it through a reverse proxy, tunnel or port forwarding unless you have reviewed the public-mode guidance.

### Docker and Hugging Face Spaces

The Docker image sets `GEOSIGNAL_PUBLIC=1` and binds `0.0.0.0:7860`. That mode is intentionally reachable by other machines. It adds:

- same-host browser origin checks for state-changing requests, including a non-default port;
- crawler rejection of loopback, private, link-local, single-label and metadata hosts, including redirects;
- a second address check at connect time, so a hostname that re-resolves to a private address after the URL check is still refused;
- public-host checks for custom model and update URLs;
- a 10-page crawl cap and a limit of three concurrent audits;
- no redirect following for update downloads.

It does **not** add authentication, user accounts or per-user data isolation. A public instance's scan history and reports are visible to every visitor. Do not use a public instance for confidential client work, and do not put secrets or private API keys in a Space.

## Security boundaries

- The HTTP body limit is 1 MB.
- Expensive work is bounded: crawls are page-capped and audit concurrency is limited, so a public instance rejects work instead of exhausting memory or threads.
- Static file serving rejects dotfiles, backslashes and path traversal.
- The UI is served with a restrictive Content Security Policy and hardening headers.
- Model downloads use HTTPS and pinned SHA-256 verification.
- Custom model files are non-executable data; GeoSignal never runs them as programs.
- Content updates are data-only and are verified against a pinned SHA-256.
- Model weights and scan data are stored outside the repository.
- The container runs as UID 1000, not root.

## Public-mode limitations

SSRF blocking and public-origin checks reduce exposure; they are not a substitute for network isolation or authentication. The concurrency cap is a resource limit, not per-client rate limiting: there is no per-IP throttling, abuse detection or account isolation. If you need a hardened multi-user service, put GeoSignal behind an authenticated reverse proxy, add rate limiting and network egress controls, or run a private instance instead.

## Dependency and container hygiene

The release build should be produced from a clean checkout with `Cargo.lock`. Rebuild the Docker image when Rust dependencies or the base image change. The repository's CI builds the library and the server binary; review dependency advisories with `cargo audit` before a release.
