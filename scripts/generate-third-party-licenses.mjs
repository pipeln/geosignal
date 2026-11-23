#!/usr/bin/env node
// Regenerate THIRD_PARTY_LICENSES.md from Cargo.lock.
//
// Usage:
//   cargo about generate --format json -o /tmp/geosignal-licenses.json
//   node scripts/generate-third-party-licenses.mjs /tmp/geosignal-licenses.json
//
// If no JSON path is supplied, cargo-about is run automatically.

import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
let jsonPath = process.argv[2];
let tempDir = null;

if (!jsonPath) {
  tempDir = mkdtempSync(join(tmpdir(), 'geosignal-licenses-'));
  jsonPath = join(tempDir, 'licenses.json');
  try {
    execFileSync(
      'cargo',
      ['about', 'generate', '--format', 'json', '--locked', '--fail', '-o', jsonPath],
      { cwd: root, stdio: 'inherit' },
    );
  } catch (error) {
    rmSync(tempDir, { recursive: true, force: true });
    throw error;
  }
}

const data = JSON.parse(readFileSync(jsonPath, 'utf8'));
if (tempDir) rmSync(tempDir, { recursive: true, force: true });
const licenses = data.licenses || [];
const crates = new Map();
for (const license of licenses) {
  for (const use of license.used_by || []) {
    const crate = use.crate || {};
    if (crate.name === 'geosignal') continue;
    if (crate.name && crate.version) {
      crates.set(`${crate.name}@${crate.version}`, crate);
    }
  }
}
const overview = new Map();
for (const license of licenses) {
  overview.set(
    license.name,
    (overview.get(license.name) || 0) + (license.used_by || []).length,
  );
}

const lines = [];
lines.push('# Third-party licenses');
lines.push('');
lines.push('GeoSignal is released under the MIT license in [LICENSE](LICENSE).');
lines.push('');
lines.push('This file covers the Rust crates and native code linked into the GeoSignal binaries. It was generated from `Cargo.lock` with `cargo-about` and contains the full license text and attribution supplied by each component. The optional GGUF model weights are **not** included; their separate terms are described in [MODEL_LICENSES.md](MODEL_LICENSES.md).');
lines.push('');
lines.push(`The current dependency graph contains ${crates.size} third-party Rust crates. A crate offered under more than one license appears once for each applicable license text; the table counts those attributions.`);
lines.push('');
lines.push('## License overview');
lines.push('');
lines.push('| License | Attributions |');
lines.push('| --- | ---: |');
for (const [name, count] of [...overview.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
  lines.push(`| ${name.replaceAll('|', '\\|')} | ${count} |`);
}
lines.push('');
lines.push('## Native components');
lines.push('');
lines.push('### llama.cpp / ggml');
lines.push('');
lines.push('The embedded inference engine is compiled from llama.cpp, including ggml. It is distributed under the MIT license:');
lines.push('');
lines.push('```text');
lines.push(readFileSync(join(root, 'licenses/llama.cpp-LICENSE.txt'), 'utf8').trimEnd());
lines.push('```');
lines.push('');
lines.push('## Full Rust license texts');
lines.push('');
for (const license of licenses) {
  const used = (license.used_by || [])
    .map((use) => use.crate)
    .filter((crate) => crate?.name && crate?.version && crate.name !== 'geosignal')
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
  if (!used.length) continue;
  lines.push(`### ${license.name}${license.id ? ` (${license.id})` : ''}`);
  lines.push('');
  lines.push('Used by:');
  lines.push('');
  for (const crate of used) {
    const url = crate.repository
      || `https://crates.io/crates/${encodeURIComponent(crate.name)}/${encodeURIComponent(crate.version)}`;
    lines.push(`- [${crate.name} ${crate.version}](${url})`);
  }
  lines.push('');
  lines.push('```text');
  lines.push(String(license.text || '').replaceAll('\r\n', '\n').trimEnd());
  lines.push('```');
  lines.push('');
}

writeFileSync(join(root, 'THIRD_PARTY_LICENSES.md'), lines.join('\n'));
console.log(
  `Wrote THIRD_PARTY_LICENSES.md (${crates.size} crates, ${licenses.length} license entries)`,
);
