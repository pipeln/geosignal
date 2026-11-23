import { HELP_TOPICS, GUIDES } from './guides.js';

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

const esc = (value) =>
  String(value ?? '').replace(/[&<>"']/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);

const STATUS_LABEL = { pass: 'Pass', warn: 'Improve', fail: 'Fail', info: 'Note' };
const CAT_LABEL = {
  technical: 'Technical',
  onpage: 'On-page',
  entity: 'Entity',
  linking: 'Linking',
  trust: 'Trust',
};
const CAT_BLURB = {
  technical: 'Crawlability, canonicalisation, mobile readiness and page experience.',
  onpage: 'Titles, headings and copy tying services to places.',
  entity: 'Who you are, where you are, and how to reach you.',
  linking: 'Location and service-area pages that are discoverable.',
  trust: 'Depth, evidence, accessibility and usefulness.',
};

const DEFAULT_OVERRIDES = { disabledRules: [], weights: {}, customRules: [], aiGuidance: '', sources: [] };

const state = {
  scan: null,
  report: null,
  models: null,
  compare: null,
  leaderboard: [],
  scans: [],
  standards: { rules: [], categories: [] },
  overrides: { ...DEFAULT_OVERRIDES },
  selected: new Set(JSON.parse(localStorage.getItem('geosignal.models') || '[]')),
  customModels: JSON.parse(localStorage.getItem('geosignal.customModels') || '[]'),
  showAllFindings: false,
  busy: false,
  comparing: false,
  aiError: '',
  timer: null,
  aiTimer: null,
  helpEnabled: localStorage.getItem('geosignal.help') !== 'off',
  updates: { appVersion: '', source: '', content: {}, lastCheck: '', lastError: '' },
  updateCheck: null,
};

/* ---------- utilities ---------- */

async function api(path, options = {}) {
  const res = await fetch(path, { headers: { 'content-type': 'application/json' }, ...options });
  const type = res.headers.get('content-type') || '';
  const data = type.includes('application/json') ? await res.json().catch(() => ({})) : {};
  if (!res.ok) throw new Error(data.error || `Request failed (${res.status})`);
  return data;
}

let toastTimer = null;
function toast(message, bad = false) {
  const el = $('#toast');
  el.textContent = message;
  el.classList.toggle('bad', Boolean(bad));
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 3600);
}

function switchTab(name) {
  $$('.tab').forEach((tab) => {
    const active = tab.dataset.tab === name;
    tab.classList.toggle('active', active);
    tab.setAttribute('aria-selected', String(active));
  });
  $$('.tab-panel').forEach((panel) => panel.classList.toggle('active', panel.id === `tab-${name}`));
  history.replaceState(null, '', `#${name}`);
  window.scrollTo({ top: 0, behavior: 'smooth' });
}

function labelFor(tag) {
  const preset = (state.models?.presets ?? []).find((item) => item.tag === tag);
  const custom = customGgufModels().find((item) => item.tag === tag);
  return preset ? preset.label : custom ? custom.label : tag;
}

/* ---------- help + fix guides ---------- */

function isDesktopShell() {
  return Boolean(window.__GEOSIGNAL_DESKTOP__);
}

function helpButton(key, label, extraClass = '') {
  const title = label || HELP_TOPICS[key]?.title || 'Explain this';
  return `<button type="button" class="help-btn${extraClass ? ` ${extraClass}` : ''}" data-help="${esc(key)}" aria-label="${esc(`Explain: ${title}`)}" title="${esc(title)}">?</button>`;
}

function decorateHelp(root = document) {
  $$('[data-help]', root).forEach((host) => {
    if (host.dataset.helpReady || host.classList.contains('help-btn')) return;
    host.dataset.helpReady = '1';
    const key = host.dataset.help;
    if (host.matches('.field')) {
      const label = host.querySelector('label');
      if (label) {
        const row = document.createElement('div');
        row.className = 'label-row';
        label.replaceWith(row);
        row.append(label);
        row.insertAdjacentHTML('beforeend', helpButton(key));
      } else {
        host.insertAdjacentHTML('afterbegin', helpButton(key));
      }
      return;
    }
    const head = host.querySelector(':scope > .section-head, :scope > .cat-head, :scope > .model-card-top, :scope > .metric-top, :scope > .ai-note-head');
    if (head) {
      head.insertAdjacentHTML('beforeend', ` ${helpButton(key)}`);
      return;
    }
    host.classList.add('has-help-corner');
    host.insertAdjacentHTML('beforeend', helpButton(key, undefined, 'help-corner'));
  });
}

function applyHelpSetting() {
  document.body.classList.toggle('help-off', !state.helpEnabled);
  const toggle = $('#help-toggle');
  if (!toggle) return;
  toggle.textContent = state.helpEnabled ? 'Tips on' : 'Tips off';
  toggle.classList.toggle('off', !state.helpEnabled);
  toggle.setAttribute('aria-pressed', String(state.helpEnabled));
}

let helpReturnFocus = null;

function openHelpPanel({ kicker = 'Quick explainer', title, html }) {
  const modal = $('#help-modal');
  if (!modal) return;
  if (modal.hidden) helpReturnFocus = document.activeElement;
  $('#help-kicker').textContent = kicker;
  $('#help-title').textContent = title;
  $('#help-body').innerHTML = html;
  modal.hidden = false;
  const card = modal.querySelector('.help-card');
  if (card) card.scrollTop = 0;
  modal.querySelector('[data-help-close]')?.focus();
}

function closeHelpPanel() {
  const modal = $('#help-modal');
  if (!modal || modal.hidden) return;
  modal.hidden = true;
  if (helpReturnFocus instanceof HTMLElement && document.contains(helpReturnFocus)) helpReturnFocus.focus();
  helpReturnFocus = null;
}

function openTopicHelp(key) {
  const topic = HELP_TOPICS[key];
  if (!topic) return;
  openHelpPanel({
    title: topic.title,
    html: `<p class="lead">${esc(topic.body)}</p>`,
  });
}

function openGuide(id) {
  const check = state.report?.checks?.find((item) => item.id === id);
  const rule = state.standards?.rules?.find((item) => item.id === id);
  const item = check || rule;
  if (!item) {
    toast('This guide is not available for the loaded report', true);
    return;
  }
  const guide = GUIDES[id] || {};
  const fallbackStep = item.fix?.trim() || 'Ask your web developer to make the change described in the report.';
  const steps = guide.steps?.length ? guide.steps : [fallbackStep];
  const sources = sourceLinks(item.sources);
  const code = guide.code
    ? `<div class="guide-code">
        <div class="guide-code-head"><span>${esc(guide.code.label || 'Code example')}</span><button type="button" class="btn ghost small" data-copy-guide>Copy</button></div>
        <pre><code id="guide-code">${esc(guide.code.snippet)}</code></pre>
      </div>`
    : '';
  const evidence = check?.evidence
    ? `<div class="guide-evidence"><strong>What GeoSignal found</strong>${esc(check.evidence)}</div>`
    : '';
  openHelpPanel({
    kicker: 'Fix guide',
    title: item.title,
    html: `
      <p class="lead">${esc(guide.plain || item.why || item.fix)}</p>
      ${evidence}
      <h4>What to do</h4>
      <ol>${steps.map((step) => `<li>${esc(step)}</li>`).join('')}</ol>
      ${code}
      <h4>Who can help</h4>
      <div class="guide-who">${esc(guide.who || 'Your web developer or whoever manages the website. Ask them to change a staging copy first, then publish it.')}</div>
      ${sources ? `<p class="guide-sources">Source: ${sources}</p>` : ''}
    `,
  });
}

async function copyGuideCode() {
  const text = $('#guide-code')?.textContent || '';
  if (!text) return;
  let copied = false;
  try {
    await navigator.clipboard.writeText(text);
    copied = true;
  } catch {
    const area = document.createElement('textarea');
    area.value = text;
    document.body.appendChild(area);
    try {
      area.select();
      copied = document.execCommand?.('copy') === true;
    } finally {
      area.remove();
    }
  }
  toast(copied ? 'Code copied' : 'Copy failed — select the code and copy it', !copied);
}

function ring(score) {
  const circumference = 2 * Math.PI * 52;
  const offset = circumference * (1 - Math.max(0, Math.min(100, score)) / 100);
  const tone = score >= 70 ? 'good' : score >= 50 ? 'mid' : 'bad';
  return `<div class="ring ${tone}">
    <svg viewBox="0 0 120 120" role="img" aria-label="Local signal score ${score} out of 100">
      <circle class="ring-track" cx="60" cy="60" r="52"></circle>
      <circle class="ring-value" cx="60" cy="60" r="52" stroke-dasharray="${circumference.toFixed(1)}" stroke-dashoffset="${offset.toFixed(1)}"></circle>
    </svg>
    <div class="ring-label"><strong>${score}</strong><span>/100</span></div>
  </div>`;
}

function bar(score) {
  return `<div class="bar ${score < 50 ? 'low' : score < 75 ? 'mid' : ''}"><i style="width:${Math.max(2, score)}%"></i></div>`;
}

function statRow(report) {
  const count = (status) => report.checks.filter((check) => check.status === status).length;
  return `<div class="stat-row">
    <div class="stat-tile"><strong>${count('pass')}</strong><span>passing</span></div>
    <div class="stat-tile"><strong>${count('warn')}</strong><span>to improve</span></div>
    <div class="stat-tile"><strong>${count('fail')}</strong><span>failing</span></div>
    <div class="stat-tile"><strong>${count('info')}</strong><span>notes</span></div>
  </div>`;
}

/* ---------- models ---------- */

function engineModels() {
  return state.models?.engine?.models ?? [];
}

function builtinModels() {
  return engineModels().filter((model) => !model.custom);
}

function customGgufModels() {
  return engineModels().filter((model) => model.custom);
}

function builtinReadyCount() {
  return builtinModels().filter((model) => model.installed).length;
}

function bytesLabel(bytes) {
  if (!bytes) return '0 MB';
  return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(2)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

function renderModelStatus() {
  const chip = $('#ai-chip');
  const models = state.models ?? {};
  const ready = engineModels().filter((model) => model.installed).length;
  const builtinReady = builtinReadyCount();
  const customReady = customGgufModels().filter((model) => model.installed).length;
  const downloading = engineModels().find((model) => model.downloading);
  if (ready) {
    chip.className = 'chip good';
    chip.textContent = `Local · ${ready} model${ready === 1 ? '' : 's'} ready`;
  } else if (downloading) {
    const pct = downloading.expectedBytes
      ? Math.min(100, Math.round(((downloading.bytes || 0) * 100) / downloading.expectedBytes))
      : 0;
    chip.className = 'chip warn';
    chip.textContent = `Downloading model · ${pct}%`;
  } else if (models.online) {
    chip.className = 'chip good';
    chip.textContent = `Ollama online · ${models.models.length} model${models.models.length === 1 ? '' : 's'}`;
  } else {
    chip.className = 'chip warn';
    chip.textContent = 'No model downloaded';
  }
  const hint = $('#model-hint');
  if (hint) {
    const total = builtinModels().length;
    if (ready) {
      const extra = customReady ? ` plus ${customReady} custom model${customReady === 1 ? '' : 's'}` : '';
      hint.textContent = `${builtinReady} of ${total} built-in models downloaded${extra}. Runs inside the app — no Ollama, no API keys.`;
    } else {
      hint.textContent = 'The rulebook and reports work without a model. Download a built-in model once in the Model lab, or add any verified GGUF model — no Ollama or LM Studio.';
    }
  }
}

function renderPickers() {
  if (!state.models) return;
  const containers = [$('#model-picker'), $('#lab-picker')].filter(Boolean);
  const list = [
    ...(state.models.presets ?? []).map((preset) => {
      const engine = builtinModels().find((model) => model.id === preset.id);
      return {
        tag: preset.tag,
        label: preset.label,
        meta: engine?.installed ? `${preset.size} · built-in` : `${preset.size} · download in Model lab`,
        installed: preset.installed,
        disabled: !preset.installed,
      };
    }),
    ...customGgufModels()
      .filter((model) => model.installed)
      .map((model) => ({
        tag: model.tag,
        label: model.label,
        meta: `${model.size} · custom GGUF`,
        installed: true,
      })),
    ...state.customModels.map((tag) => ({
      tag,
      label: tag,
      meta: 'Custom Ollama tag',
      installed: (state.models.models ?? []).some((model) => model.name === tag || model.name === `${tag}:latest`),
    })),
  ];
  const html = list.length
    ? list
        .map(
          (item) => `<label class="model-option ${item.installed ? '' : 'missing'}">
        <input type="checkbox" value="${esc(item.tag)}" ${state.selected.has(item.tag) ? 'checked' : ''} ${item.disabled ? 'disabled' : ''}>
        <span class="mo-body">
          <span class="mo-name">${esc(item.label)}${item.installed ? '' : ' <em>not downloaded</em>'}</span>
          <span class="mo-meta">${esc(item.meta)}</span>
        </span>
      </label>`,
        )
        .join('')
    : '<p class="hint">No model presets available.</p>';

  containers.forEach((container) => {
    container.innerHTML = html;
    container.querySelectorAll('input[type="checkbox"]').forEach((input) => {
      input.addEventListener('change', () => {
        if (input.checked) state.selected.add(input.value);
        else state.selected.delete(input.value);
        localStorage.setItem('geosignal.models', JSON.stringify([...state.selected]));
        containers.forEach((other) => {
          other.querySelectorAll('input[type="checkbox"]').forEach((box) => {
            if (box.value === input.value) box.checked = input.checked;
          });
        });
      });
    });
  });
}

async function loadModels() {
  try {
    state.models = await api('/api/models');
  } catch (error) {
    state.models = { online: false, presets: [], models: [], error: error.message };
  }
  if (state.selected.size === 0) {
    (state.models.presets ?? []).filter((preset) => preset.installed).forEach((preset) => state.selected.add(preset.tag));
    customGgufModels()
      .filter((model) => model.installed)
      .forEach((model) => state.selected.add(model.tag));
  }
  renderModelStatus();
  renderPickers();
}

let engineTimer = null;

async function refreshEngine() {
  try {
    const engine = await api('/api/engine');
    if (state.models) state.models.engine = engine;
    (state.models?.presets ?? []).forEach((preset) => {
      const model = (engine.models ?? []).find((item) => item.id === preset.id);
      if (model) {
        preset.embedded = model.installed;
        preset.installed = preset.installed || model.installed;
      }
    });
  } catch {
    // Engine status is non-critical; keep the last known state.
  }
}

function pollEngine() {
  clearInterval(engineTimer);
  engineTimer = setInterval(async () => {
    await refreshEngine();
    renderLab();
    renderModelStatus();
    renderPickers();
    if (!engineModels().some((model) => model.downloading)) {
      clearInterval(engineTimer);
      await loadModels();
      renderLab();
    }
  }, 900);
}

/* ---------- scanning ---------- */

function setScanBusy(busy) {
  state.busy = busy;
  const button = $('#scan-submit');
  if (!button) return;
  button.disabled = busy;
  button.innerHTML = busy ? '<span class="spin"></span>Crawling…' : 'Run quick check';
}

function startTimer(message, element) {
  const el = element || $('#scan-status');
  if (!el) return;
  const started = Date.now();
  el.classList.remove('error');
  const paint = () => {
    el.innerHTML = `<span class="spin"></span>${esc(message)} ${Math.round((Date.now() - started) / 1000)}s`;
  };
  paint();
  clearInterval(state.timer);
  state.timer = setInterval(paint, 500);
}

function stopTimer() {
  clearInterval(state.timer);
  state.timer = null;
}

function setStatus(message, isError = false, element) {
  const el = element || $('#scan-status');
  if (!el) return;
  el.textContent = message;
  el.classList.toggle('error', isError);
}

async function runScan(event) {
  event?.preventDefault();
  if (state.busy) return;
  const address = $('#url').value.trim();
  if (!address) {
    setStatus('Enter a website address first.', true);
    $('#url').focus();
    return;
  }
  const payload = {
    url: address,
    keyword: $('#keyword').value.trim(),
    city: $('#city').value.trim(),
    maxPages: Number($('#max-pages').value),
  };
  setScanBusy(true);
  startTimer('Crawling site');
  try {
    const data = await api('/api/scan', { method: 'POST', body: JSON.stringify(payload) });
    state.scan = data.scan;
    state.report = data.report;
    state.compare = null;
    state.aiError = '';
    state.showAllFindings = false;
    stopTimer();
    setStatus(`Tested ${data.scan.pages.length} page(s) in ${(data.scan.durationMs / 1000).toFixed(1)}s · ${data.report.score}/100.`);
    renderQuickResult();
    renderReport();
    loadScans();
    switchTab('report');
    toast(`Check complete — ${data.report.score}/100 (${data.report.band})`);
  } catch (error) {
    stopTimer();
    setStatus(error.message, true);
    toast(error.message, true);
  } finally {
    setScanBusy(false);
  }
}

function renderQuickResult() {
  const root = $('#quick-result');
  if (!root) return;
  if (!state.scan || !state.report) {
    root.innerHTML = '';
    return;
  }
  const { scan, report } = state;
  const top = report.fixes.slice(0, 3);
  root.innerHTML = `<section class="card pad" data-help="quick_result" style="margin-bottom:18px">
    <div class="section-head">
      <div>
        <h3>Latest check: ${esc(scan.host)}</h3>
        <p class="hint">${esc(report.band)} · ${report.score}/100 · ${scan.pages.length} page(s) tested</p>
      </div>
      <button class="btn secondary" data-action="open-report">Open full report</button>
    </div>
    <ul class="steps">
      ${top.map((check) => `<li><strong>${esc(check.title)}</strong> — ${esc(check.evidence)} <button type="button" class="btn ghost small" data-guide="${esc(check.id)}">How to fix</button></li>`).join('') || '<li>No priority fixes found on the crawled pages.</li>'}
    </ul>
  </section>`;
  decorateHelp(root);
}

/* ---------- report ---------- */

function sourceLinks(sources) {
  return (sources ?? [])
    .filter((source) => source?.url)
    .map((source) => `<a href="${esc(source.url)}" target="_blank" rel="noreferrer">${esc(source.label)}</a>`)
    .join(' · ');
}

function sourcesLine(check) {
  const links = sourceLinks(check.sources);
  return links ? `<p class="sources">${links}</p>` : '';
}

function actionHeadline(text, max = 92) {
  const clean = String(text || '').replace(/\s+/g, ' ').trim();
  if (clean.length <= max) return clean;
  const cut = clean.slice(0, max);
  const space = cut.lastIndexOf(' ');
  return `${cut.slice(0, space > 40 ? space : max).replace(/[,;:(]+$/, '').trimEnd()}…`;
}

function findingRow(check, open = false) {
  const passing = check.status === 'pass';
  const headline = passing ? check.title : actionHeadline(check.fix);
  return `<details class="finding ${check.status}" ${open ? 'open' : ''}>
    <summary>
      <span class="badge ${check.status}">${STATUS_LABEL[check.status]}</span>
      <span class="finding-title">${esc(headline)}</span>
      <span class="check-cat">${esc(CAT_LABEL[check.category] || check.category)}</span>
      <span class="weight-tag">weight ${check.weight}</span>
    </summary>
    <div class="finding-body">
      ${passing ? '' : `<p><span class="label">Standard</span>${esc(check.title)}</p>`}
      <p><span class="label">Evidence</span>${esc(check.evidence)}</p>
      <p><span class="label">${passing ? 'How to keep it' : 'Fix'}</span>${esc(check.fix)}</p>
      <p class="why"><span class="label">Why it ${passing ? 'works' : 'matters'}</span>${esc(check.why)}</p>
      ${sourcesLine(check)}
      <div class="help-actions">
        <button type="button" class="btn secondary small" data-guide="${esc(check.id)}">How to fix this</button>
        <span class="hint">plain-language steps, copyable code and who to call</span>
      </div>
    </div>
  </details>`;
}

function checkLedger(checks) {
  return `<table class="compact-table">
    <thead><tr><th>Result</th><th>Check</th><th>Evidence</th></tr></thead>
    <tbody>
      ${checks
        .map(
          (check) => `<tr>
          <td><span class="dot ${check.status}"></span>${STATUS_LABEL[check.status]}</td>
          <td>${esc(check.title)}</td>
          <td>${esc(check.evidence)}</td>
        </tr>`,
        )
        .join('')}
    </tbody>
  </table>`;
}

function categoryStrip(report) {
  return `<div class="cat-grid">
    ${report.categories
      .map(
        (category) => `<div class="card cat-card" data-help="report_categories">
        <div class="cat-head"><h4>${esc(category.label)}</h4><span class="cat-score">${category.score}%</span></div>
        ${bar(category.score)}
        <p class="hint">${category.passed}/${category.total} passing · ${esc(CAT_BLURB[category.key] || '')}</p>
      </div>`,
      )
      .join('')}
  </div>`;
}

function entityCard(scan) {
  const business = scan.signals.business ?? {};
  const address = [business.street, business.locality, business.region].filter(Boolean).join(', ');
  const items = [
    ['Name', business.name || 'Not detected', 'entity_name'],
    ['Schema', (business.types || []).join(', ') || 'No LocalBusiness markup', 'entity_schema'],
    ['Address', address || 'Not in markup', 'entity_address'],
    ['Phone', business.phone || 'Not in markup', 'entity_phone'],
    ['Areas served', (business.areaServed || []).slice(0, 4).join(', ') || 'Not set', 'entity_areas'],
    ['Sitemap', scan.signals.sitemapFound ? `${scan.signals.sitemapUrls} URLs` : 'Not found', 'entity_sitemap'],
  ];
  return `<section class="card section-card" data-help="report_entity">
    <div class="section-head"><h3>Local entity</h3><span class="hint">what search engines can verify</span></div>
    <div class="entity-grid compact">
      ${items.map(([label, value, key]) => `<div class="entity-item"><span class="label">${esc(label)}</span><span class="value">${esc(value)} ${helpButton(key)}</span></div>`).join('')}
    </div>
  </section>`;
}

function pagesCard(scan) {
  return `<section class="card section-card" data-help="report_pages">
    <details class="plain-details">
      <summary><strong>Crawled pages</strong> <span class="hint">${scan.pages.length} page(s) · ${scan.signals.internalLinkTotal} internal links</span></summary>
      <div class="table-wrap">
        <table class="compact-table">
          <thead><tr><th>Page</th><th>Depth</th><th>Status</th><th>Words</th><th>Links</th><th>Indexable</th></tr></thead>
          <tbody>
            ${scan.pages
              .map(
                (page) => `<tr>
                <td class="path">${esc(page.path)}</td>
                <td>${page.depth}</td>
                <td>${page.status}</td>
                <td>${page.wordCount}</td>
                <td>${page.links.internal}</td>
                <td>${page.noindex ? '<span class="pill-no">noindex</span>' : 'yes'}</td>
              </tr>`,
              )
              .join('')}
          </tbody>
        </table>
      </div>
    </details>
  </section>`;
}

function methodCard() {
  return `<section class="card section-card" data-help="report_method">
    <details class="plain-details">
      <summary><strong>Method &amp; limits</strong></summary>
      <div class="method-body">
        <p><strong>Measured.</strong> 34 documented checks across local entity, on-page relevance, location linking, technical access and content trust. Every check carries its evidence, the mechanism and a Google source.</p>
        <p><strong>Not measured.</strong> Actual rankings, impressions, clicks, competitor visibility and Business Profile performance. Those live in Search Console, Business Profile Insights and rank trackers.</p>
        <p><strong>Core Web Vitals.</strong> The crawl sees proxies only. Field thresholds to confirm: LCP ≤ 2.5 s, INP ≤ 200 ms, CLS ≤ 0.1 at p75.</p>
        <p class="hint">References:
          <a href="https://developers.google.com/search/docs/essentials" target="_blank" rel="noreferrer">Search Essentials</a> ·
          <a href="https://support.google.com/business/answer/7091" target="_blank" rel="noreferrer">Local ranking tips</a> ·
          <a href="https://developers.google.com/search/docs/appearance/structured-data/local-business" target="_blank" rel="noreferrer">LocalBusiness data</a> ·
          <a href="https://developers.google.com/search/docs/appearance/core-web-vitals" target="_blank" rel="noreferrer">Core Web Vitals</a>
        </p>
      </div>
    </details>
  </section>`;
}

function metricCards(compare) {
  const best = Math.max(...compare.metrics.map((metric) => metric.score));
  return compare.metrics
    .map((metric) => {
      const run = compare.runs.find((item) => item.model === metric.model);
      const isBest = metric.score > 0 && metric.score === best && compare.metrics.length > 1;
      return `<div class="metric-card ${isBest ? 'winner' : ''}" data-help="lab_compare">
      ${isBest ? '<span class="winner-tag">Best run</span>' : ''}
      <div class="metric-top"><h4>${esc(metric.label)}</h4><span class="metric-score">${metric.score}</span></div>
      <div class="metric-rows">
        <div><span>Valid JSON</span><b>${metric.validJson ? 'yes' : 'no'}</b></div>
        <div><span>Latency</span><b>${(metric.latencyMs / 1000).toFixed(1)}s</b></div>
        <div><span>Suggestions</span><b>${metric.suggestions}</b></div>
        <div><span>Schema</span><b>${metric.schemaCompliance}%</b></div>
        <div><span>Agreement</span><b>${metric.consensusMatch}%</b></div>
      </div>
      ${run?.error ? `<p class="hint" style="margin-top:8px;color:var(--red)">${esc(run.error)}</p>` : ''}
    </div>`;
    })
    .join('');
}

function aiNote(item, compare, unique = false) {
  const total = compare.runs.length;
  return `<article class="ai-note">
    <div class="ai-note-head">
      <strong>${esc(item.title)} ${helpButton('report_ai')}</strong>
      <span class="votes" title="${item.voteCount} of ${total} models">${'●'.repeat(item.voteCount)}${'○'.repeat(Math.max(0, total - item.voteCount))}</span>
    </div>
    <p class="meta-line">${esc(item.category)} · ${esc(item.severity)} · impact ${item.impact}/5 · effort ${esc(item.effort)}</p>
    <p>${esc(item.evidence)}</p>
    <details><summary>Fix and why</summary>
      <p><span class="label">Fix</span>${esc(item.fix)}</p>
      <p class="why"><span class="label">Why it works</span>${esc(item.why)}</p>
      <p class="hint">${unique ? 'Raised only by' : 'Agreed by'} ${item.votes.map(esc).join(', ')} · avg confidence ${item.confidence}%</p>
    </details>
  </article>`;
}

function aiPanel() {
  if (!state.scan) return '';
  const selected = [...state.selected];
  if (!state.compare) {
    return `<section class="card section-card" data-help="report_ai">
      <div class="section-head"><h3>Model cross-check</h3><span class="hint">optional · built-in models</span></div>
      <p class="section-sub">Ask one or more small models to review this crawl independently. Answers are grouped by issue and ranked by agreement, so disagreement is visible.</p>
      <div class="ai-intro">${
        selected.length
          ? selected.map((tag) => `<span class="chip">${esc(labelFor(tag))}</span>`).join('')
          : '<span class="chip warn">No model selected</span>'
      }</div>
      <button class="btn primary" data-action="run-ai">Run ${selected.length || ''} model${selected.length === 1 ? '' : 's'} on this scan</button>
      <span id="ai-status" class="status" style="margin-left:10px"></span>
      ${state.aiError ? `<p class="status error" style="margin-top:10px">${esc(state.aiError)}</p>` : ''}
      <p class="hint" style="margin-top:10px">The rulebook score is the source of truth. Anything a model claims without crawl evidence is noise.</p>
    </section>`;
  }
  const compare = state.compare;
  return `<section class="card section-card" data-help="report_ai">
    <div class="section-head">
      <h3>Model cross-check</h3>
      <span class="hint">${esc(compare.mode)} · ${compare.runs.length} model(s) · ${new Date(compare.at).toLocaleTimeString()}</span>
    </div>
    <div class="metric-grid">${metricCards(compare)}</div>
    <h4>Consensus findings</h4>
    ${compare.curated.slice(0, 3).map((item) => aiNote(item, compare)).join('') || '<p class="hint">No suggestions could be parsed. Check the model errors above.</p>'}
    ${
      compare.curated.length > 3
        ? `<details class="plain-details"><summary>${compare.curated.length - 3} more consensus finding(s)</summary>${compare.curated
            .slice(3)
            .map((item) => aiNote(item, compare))
            .join('')}</details>`
        : ''
    }
    ${
      compare.unique.length
        ? `<details class="plain-details"><summary>${compare.unique.length} model-specific insight(s)</summary>${compare.unique
            .map((item) => aiNote(item, compare, true))
            .join('')}</details>`
        : ''
    }
    <div class="export-row">
      <button class="btn secondary small" data-action="run-ai">Run again</button>
      <button class="btn ghost small" data-action="open-lab">Open model lab</button>
    </div>
  </section>`;
}

function renderReport() {
  const root = $('#report-root');
  if (!root) return;
  if (!state.scan || !state.report) {
    root.innerHTML = `<div class="empty-state card">
      <h2>No report yet</h2>
      <p>Run a quick check and the tested results appear here: score, category breakdown, priority findings and the reasoning behind each signal.</p>
      <button class="btn primary" data-action="go-scan">Run a quick check</button>
    </div>`;
    return;
  }
  const { scan, report } = state;
  const visible = state.showAllFindings ? report.fixes : report.fixes.slice(0, 6);

  root.innerHTML = `
    <div class="report-head card">
      <div class="report-title">
        <p class="eyebrow">Quick check report</p>
        <h2>${esc(scan.host)}</h2>
        <p class="report-meta">${esc(scan.url)} · ${scan.pages.length} page(s) tested · ${new Date(scan.finishedAt).toLocaleString()}</p>
        <div class="ai-intro">
          <span class="chip">${esc(scan.target.keyword || 'Service not set')}</span>
          <span class="chip">${esc(scan.target.city || 'City not detected')}</span>
        </div>
      </div>
      <div class="score-wrap">${ring(report.score)}<span class="band">${esc(report.band)}</span></div>
      <div class="report-actions">
        <button class="btn primary" data-action="export-html">Full report</button>
        <button class="btn secondary" data-action="export-md">Markdown</button>
        <button class="btn ghost" data-action="rescan">Re-scan</button>
        ${helpButton('report_score', 'Explain the score')}
      </div>
      ${statRow(report)}
    </div>

    <div class="notice" data-help="report_score">
      <div>
        <strong>Ranking readiness, not a ranking position.</strong>
        ${report.checks.length} documented checks tested from this crawl. Real positions and impressions come from Search Console or a rank tracker — this tells you what to fix and why.
      </div>
    </div>

    ${categoryStrip(report)}

    <section class="card section-card" data-help="report_findings">
      <div class="section-head"><h3>Findings</h3><span class="hint">${report.fixes.length} need attention</span></div>
      <p class="section-sub">Ordered by severity and signal weight. Expand a row for the evidence, fix and mechanism.</p>
      ${visible.map((check, index) => findingRow(check, index === 0 && !state.showAllFindings)).join('') || '<p class="hint">No failing or warning checks on the crawled pages.</p>'}
      ${
        report.fixes.length > 6
          ? `<button class="btn ghost small more-btn" data-action="toggle-findings">${state.showAllFindings ? 'Show fewer' : `Show all ${report.fixes.length} findings`}</button>`
          : ''
      }
    </section>

    <section class="card section-card" data-help="report_tested">
      <div class="section-head"><h3>Tested checks</h3><span class="hint">${report.working.length} passing · ${report.checks.length} total</span></div>
      <div class="chip-list">
        ${report.working.map((check) => `<button type="button" class="check-chip" data-guide="${esc(check.id)}" aria-label="Open the fix guide: ${esc(check.title)}" title="Open the fix guide">${esc(check.title)}<span class="chip-help" aria-hidden="true">?</span></button>`).join('') || '<span class="hint">Nothing passing yet.</span>'}
      </div>
      <details class="plain-details">
        <summary>Open the test ledger · why each check passes</summary>
        ${checkLedger(report.checks)}
      </details>
    </section>

    ${entityCard(scan)}
    ${aiPanel()}
    ${pagesCard(scan)}
    ${methodCard()}
  `;
  decorateHelp(root);
}

/* ---------- comparison ---------- */

function startAiTimer(el, message) {
  if (!el) return;
  const started = Date.now();
  el.classList.remove('error');
  const paint = () => {
    el.innerHTML = `<span class="spin"></span>${esc(message)} ${Math.round((Date.now() - started) / 1000)}s`;
  };
  paint();
  clearInterval(state.aiTimer);
  state.aiTimer = setInterval(paint, 600);
}

async function runCompare(button) {
  if (!state.scan) {
    toast('Run a scan first', true);
    return;
  }
  const models = [...state.selected];
  if (!models.length) {
    toast('Select at least one model in the Quick check or Model lab', true);
    return;
  }
  if (state.comparing) return;
  state.comparing = true;
  if (button) button.disabled = true;
  const mode = $('#mode')?.value === 'sequential' ? 'sequential' : 'parallel';
  const statusEl = $('#ai-status') || $('#lab-ai-status');
  startAiTimer(statusEl, `${models.length} model${models.length === 1 ? '' : 's'} thinking`);
  try {
    const data = await api('/api/ai/compare', {
      method: 'POST',
      body: JSON.stringify({ scanId: state.scan.id, models, mode }),
    });
    state.compare = data.compare;
    state.leaderboard = data.leaderboard || [];
    state.aiError = '';
    clearInterval(state.aiTimer);
    renderReport();
    renderLab();
    const answered = state.compare.runs.filter((run) => run.ok).length;
    if (!answered) {
      toast('No model answered — fix the runtime error shown above', true);
    } else {
      toast(`${answered} of ${state.compare.runs.length} model(s) answered — curated by agreement`);
    }
  } catch (error) {
    clearInterval(state.aiTimer);
    state.aiError = error.message;
    renderReport();
    renderLab();
    toast(error.message, true);
  } finally {
    state.comparing = false;
    if (button) button.disabled = false;
  }
}

/* ---------- scans ---------- */

async function loadScans() {
  try {
    const data = await api('/api/scans');
    state.scans = data.scans || [];
    renderRecent();
  } catch {
    // History is non-critical.
  }
}

function renderRecent() {
  const root = $('#recent-scans');
  if (!root) return;
  const scans = (state.scans ?? []).slice(0, 5);
  root.innerHTML = scans.length
    ? scans
        .map(
          (scan) => `<button class="recent" data-scan="${esc(scan.id)}">
          <span class="recent-score">${scan.score}</span>
          <span class="recent-meta"><strong>${esc(scan.host)}</strong><span>${scan.pages} page(s) · ${new Date(scan.finishedAt).toLocaleDateString()}</span></span>
        </button>`,
        )
        .join('')
    : '<p class="hint">No scans yet.</p>';
}

async function openScan(id) {
  try {
    const data = await api(`/api/scan/${id}`);
    state.scan = data.scan;
    state.report = data.report;
    state.compare = null;
    state.aiError = '';
    state.showAllFindings = false;
    renderQuickResult();
    renderReport();
    $('#url').value = data.scan.url;
    $('#keyword').value = data.scan.target.keyword || '';
    $('#city').value = data.scan.target.city || '';
    switchTab('report');
  } catch (error) {
    toast(error.message, true);
  }
}

/* ---------- lab ---------- */

function leaderboardTable(rows) {
  return `<div class="table-wrap"><table class="leaderboard">
    <thead><tr><th></th><th>Model</th><th>Runs</th><th>Valid JSON</th><th>Avg latency</th><th>Schema</th><th>Agreement</th><th>Score</th></tr></thead>
    <tbody>
      ${rows
        .map(
          (row, index) => `<tr class="${index === 0 ? 'win' : ''}">
          <td class="rank">${index + 1}</td>
          <td><strong>${esc(row.label)}</strong><br><span class="hint">${esc(row.model)}</span></td>
          <td>${row.runs}</td>
          <td>${row.validJsonRate}%</td>
          <td>${(row.avgLatencyMs / 1000).toFixed(1)}s</td>
          <td>${row.avgSchema}%</td>
          <td>${row.avgConsensus}%</td>
          <td><strong>${row.score}</strong></td>
        </tr>`,
        )
        .join('')}
    </tbody>
  </table></div>`;
}

function renderLab() {
  const root = $('#lab-root');
  if (!root) return;
  const models = state.models ?? {};
  const cards = (models.presets ?? [])
    .map((preset) => {
      const engine = builtinModels().find((model) => model.id === preset.id) || {};
      const pct = engine.expectedBytes
        ? Math.min(100, Math.round(((engine.bytes || 0) * 100) / engine.expectedBytes))
        : 0;
      const badge = engine.installed
        ? '<span class="chip good">ready</span>'
        : engine.downloading
          ? `<span class="chip warn">${pct}%</span>`
          : engine.error
            ? '<span class="chip bad">download failed</span>'
            : '<span class="chip warn">not downloaded</span>';
      const body = engine.installed
        ? `<p class="size">Stored in <code>${esc(engine.path || 'data/models')}</code> · ${esc(preset.license)}</p>`
        : engine.downloading
          ? `<div class="progress"><span style="width:${pct}%"></span></div>
             <p class="size">${bytesLabel(engine.bytes)} of ${bytesLabel(engine.expectedBytes)} · ${pct}%</p>`
          : `<p>${esc(preset.notes)}</p>
             ${engine.error ? `<p class="hint error-text">${esc(engine.error)}</p>` : ''}
             <button class="btn secondary small" data-action="download-model" data-id="${esc(preset.id)}">Download ${esc(preset.size)}</button>
             <p class="size">${esc(preset.license)} · runs on this machine, no account</p>`;
      return `<div class="card model-card" data-help="lab_model">
      <div class="model-card-top">
        <div><h4>${esc(preset.label)}</h4><span class="size">${esc(preset.family)} · ${esc(preset.size)}</span></div>
        ${badge}
      </div>
      ${body}
    </div>`;
    })
    .join('');

  const customCards = customGgufModels()
    .map((model) => {
      const pct = model.expectedBytes
        ? Math.min(100, Math.round(((model.bytes || 0) * 100) / model.expectedBytes))
        : 0;
      const badge = model.installed
        ? '<span class="chip good">ready</span>'
        : model.downloading
          ? `<span class="chip warn">${pct}%</span>`
          : model.error
            ? '<span class="chip bad">download failed</span>'
            : '<span class="chip warn">not downloaded</span>';
      const body = model.installed
        ? `<p class="size">Stored in <code>${esc(model.path || 'data/models')}</code> · ${esc(model.license)}</p>
           <button class="btn ghost small" data-action="remove-custom-model" data-id="${esc(model.id)}">Remove model</button>`
        : model.downloading
          ? `<div class="progress"><span style="width:${pct}%"></span></div>
             <p class="size">${bytesLabel(model.bytes)} of ${bytesLabel(model.expectedBytes)} · ${pct}%</p>`
          : `<p>${esc(model.notes)}</p>
             ${model.error ? `<p class="hint error-text">${esc(model.error)}</p>` : ''}
             <button class="btn secondary small" data-action="download-model" data-id="${esc(model.id)}">Download${model.size === 'Unknown size' ? '' : ` ${esc(model.size)}`}</button>
             <button class="btn ghost small" data-action="remove-custom-model" data-id="${esc(model.id)}">Remove</button>
             <p class="size">${esc(model.license)} · verified by SHA-256</p>`;
      return `<div class="card model-card" data-help="lab_custom_model">
        <div class="model-card-top">
          <div><h4>${esc(model.label)}</h4><span class="size">${esc(model.family)} · ${esc(model.size)}</span></div>
          ${badge}
        </div>
        ${body}
      </div>`;
    })
    .join('');

  root.innerHTML = `
    <div class="hero">
      <p class="eyebrow">Model lab</p>
      <h2>Small models, scored against each other.</h2>
      <p class="lede">Run one model or three. Every answer is validated against a fixed schema, then compared. The leaderboard accumulates, so you can see which model performs best on the sites you audit.</p>
    </div>

    <section class="card section-card" data-help="lab_models">
      <div class="section-head">
        <h3>Built-in models</h3>
        <span class="chip good">llama.cpp embedded</span>
      </div>
      <p class="section-sub">Download a preset once and it runs inside GeoSignal. Weights live in <code>${esc(models.engine?.modelsDir || 'data/models')}</code> — no Ollama, no LM Studio, no API keys, no network calls after the download.</p>
      <div class="lab-grid">${cards}</div>
      <details class="plain-details">
        <summary>Ollama (optional) — ${models.online ? `online · ${models.models.length} model(s)` : 'not running'}</summary>
        <p class="hint">${models.online ? `Connected to ${esc(models.host)}.` : `No server at ${esc(models.host || 'http://127.0.0.1:11434')}.`} Ollama is only needed if you want a custom tag outside the three built-in presets.</p>
        <div class="form-grid">
          <div class="field"><label for="custom-model">Add another Ollama model tag</label><input id="custom-model" type="text" placeholder="phi4-mini" spellcheck="false"></div>
          <div class="field" style="justify-content:flex-end"><button class="btn secondary" data-action="add-model">Add to selection</button></div>
          <div class="field" style="justify-content:flex-end"><button class="btn ghost" data-action="recheck-models">Re-check runtime</button></div>
        </div>
      </details>
      <p class="hint">1–3B quantised models are the sweet spot for this task. Downloads resume if interrupted and stay on this machine.</p>
    </section>

    <section class="card section-card" data-help="lab_custom_models">
      <div class="section-head"><h3>Custom GGUF models</h3><span class="chip good">bring your own</span></div>
      <p class="section-sub">Add any GGUF model that is not in the built-in catalog. Paste its HTTPS download URL and the SHA-256 of the exact file; GeoSignal downloads it locally and refuses a file that does not match. Model licensing stays with the upstream publisher.</p>
      <div class="lab-grid">${customCards || '<p class="hint">No custom models yet. Add one below.</p>'}</div>
      <details class="plain-details">
        <summary>Add a custom GGUF model</summary>
        <div class="form-grid">
          <div class="field full"><label for="custom-gguf-label">Display name</label><input id="custom-gguf-label" type="text" placeholder="My fine-tuned Qwen 3B"></div>
          <div class="field full"><label for="custom-gguf-url">GGUF download URL (HTTPS)</label><input id="custom-gguf-url" type="url" placeholder="https://huggingface.co/you/model/resolve/main/model.gguf" spellcheck="false"></div>
          <div class="field full"><label for="custom-gguf-sha">SHA-256 of the exact file</label><input id="custom-gguf-sha" type="text" placeholder="64 hexadecimal characters" spellcheck="false"></div>
          <div class="field"><label for="custom-gguf-license">License</label><input id="custom-gguf-license" type="text" placeholder="Apache-2.0"></div>
          <div class="field"><label for="custom-gguf-family">Family <span>optional</span></label><input id="custom-gguf-family" type="text" placeholder="Qwen"></div>
          <div class="field"><label for="custom-gguf-size">Size <span>optional</span></label><input id="custom-gguf-size" type="text" placeholder="1.5 GB"></div>
          <div class="field full"><label for="custom-gguf-notes">Notes <span>optional</span></label><input id="custom-gguf-notes" type="text" placeholder="What this model is good at"></div>
          <div class="field full"><button class="btn secondary" data-action="add-custom-model">Add custom model</button><p class="hint">The URL and hash are checked again on every download. A failed hash deletes the partial file.</p></div>
        </div>
      </details>
    </section>

    <section class="card section-card" data-help="lab_compare">
      <div class="section-head"><h3>Comparison</h3><span class="hint">${state.scan ? `scan: ${esc(state.scan.host)}` : 'no scan loaded'}</span></div>
      <p class="section-sub">Select models, then run them against the loaded scan. Parallel starts all at once; sequential is gentler on RAM.</p>
      <div id="lab-picker" class="model-picker"></div>
      <div class="export-row" style="margin-top:14px">
        <button class="btn primary" data-action="run-ai" ${state.scan ? '' : 'disabled'}>Run selected models</button>
        <span id="lab-ai-status" class="status"></span>
      </div>
      ${state.compare ? `<div class="metric-grid" style="margin-top:18px">${metricCards(state.compare)}</div>` : '<p class="hint" style="margin-top:12px">Run a scan, choose models and start a comparison. Per-model metrics appear here.</p>'}
    </section>

    <section class="card section-card" data-help="lab_leaderboard">
      <div class="section-head"><h3>Leaderboard</h3><span class="hint">${(state.leaderboard || []).reduce((sum, row) => sum + row.runs, 0)} model run(s)</span></div>
      ${state.leaderboard?.length ? leaderboardTable(state.leaderboard) : '<p class="hint">No comparisons recorded yet.</p>'}
      <p class="hint" style="margin-top:12px">Score = 25% valid JSON + 20% schema completeness + 20% agreement + 25% speed + 10% sensible suggestion count (3–8). Single-model runs have no agreement baseline.</p>
    </section>

    ${
      state.compare
        ? `<section class="card section-card" data-help="lab_latest">
        <div class="section-head"><h3>Latest curated run</h3><span class="hint">${esc(state.compare.mode)} · ${state.compare.runs.length} model(s)</span></div>
        ${state.compare.curated.slice(0, 3).map((item) => aiNote(item, state.compare)).join('') || '<p class="hint">No parsed suggestions in this run.</p>'}
      </section>`
        : ''
    }
  `;
  renderPickers();
  decorateHelp(root);
}

/* ---------- standards ---------- */

function renderStandards() {
  const root = $('#standards-root');
  if (!root) return;
  const overrides = state.overrides;
  const categories = state.standards.categories ?? [];
  const rules = state.standards.rules ?? [];

  const groups = categories
    .map((category) => {
      const items = rules.filter((rule) => rule.category === category.key);
      if (!items.length) return '';
      const enabled = items.filter((rule) => !overrides.disabledRules.includes(rule.id)).length;
      return `<div class="rule-group">
        <div class="rule-group-head">
          <h4>${esc(category.label)}</h4>
          <span class="rule-count">${enabled} of ${items.length} on</span>
        </div>
        <p class="hint">${esc(category.blurb)}</p>
        <div class="rule-columns" aria-hidden="true"><span>On</span><span>Check &amp; what good looks like</span><span>Weight</span></div>
        ${items
          .map((rule) => {
            const off = overrides.disabledRules.includes(rule.id);
            const weight = overrides.weights[rule.id] ?? rule.weight;
            const sources = rule.sources ?? [];
            return `<div class="rule-row ${off ? 'off' : ''}" data-rule-row="${esc(rule.id)}">
            <label class="switch" title="Enable or disable this check">
              <input type="checkbox" class="rule-toggle" data-id="${esc(rule.id)}" ${off ? '' : 'checked'} aria-label="Enable ${esc(rule.title)}">
              <span></span>
            </label>
            <div class="rule-main">
              <div class="rule-title"><strong>${esc(rule.title)}</strong> <button type="button" class="help-btn" data-guide="${esc(rule.id)}" aria-label="Open the fix guide" title="Open the fix guide">?</button></div>
              <p class="rule-why">${esc(rule.why)}</p>
              <details class="rule-detail">
                <summary>Why this matters &amp; how to fix</summary>
                <p>${esc(rule.why)}</p>
                <p class="rule-fix"><span class="label">Fix</span>${esc(rule.fix)}</p>
                <p><button type="button" class="btn secondary small" data-guide="${esc(rule.id)}">Open the full fix guide</button></p>
                ${sources.length ? `<p class="rule-sources">Source: ${sources.map((source) => `<a href="${esc(source.url)}" target="_blank" rel="noreferrer">${esc(source.label)}</a>`).join(' · ')}</p>` : ''}
              </details>
            </div>
            <div class="weight-control">
              <input class="weight-input" type="number" min="0.5" max="5" step="0.5" value="${weight}" data-id="${esc(rule.id)}" data-default="${rule.weight}" aria-label="Weight for ${esc(rule.title)}">
              <button type="button" class="weight-default" title="Reset to the built-in weight">default ${rule.weight}</button>
            </div>
          </div>`;
          })
          .join('')}
      </div>`;
    })
    .join('');

  const custom = overrides.customRules ?? [];
  const sources = overrides.sources ?? [];

  root.innerHTML = `
    <div class="hero">
      <p class="eyebrow">Standards &amp; overrides</p>
      <h2>Your audit, your rulebook.</h2>
      <p class="lede">Everything is grounded in public Google guidance, but local markets differ. Disable checks that do not apply, re-weight what matters, add your own regex checks, and give the models extra context.</p>
    </div>

    <section class="card section-card" data-help="standards_checks">
      <div class="section-head"><h3>Checks &amp; weights</h3><span class="hint">${rules.length} built-in checks</span></div>
      <p class="section-sub">Turning a check off removes it from the score entirely. Weight controls how much it counts, and every row explains the standard, the reason and the fix. Changes recalculate the open report when saved.</p>
      <div class="weight-legend">
        <span><b>0.5</b> minor polish</span>
        <span><b>1–2</b> supporting</span>
        <span><b>3</b> standard</span>
        <span><b>4–5</b> critical</span>
        <span><b>Off</b> excluded from scoring</span>
      </div>
      ${groups || '<p class="hint">Checks could not be loaded from the server.</p>'}
    </section>

    <section class="card section-card" data-help="standards_custom">
      <div class="section-head"><h3>Custom checks</h3><span class="hint">your own rules</span></div>
      <p class="section-sub">A regex check tests page titles and visible copy. Use it for local compliance details, brand requirements, or phrases you must never publish.</p>
      ${
        custom.length
          ? custom
              .map(
                (rule) => `<div class="custom-rule">
                <div>
                  <strong>${esc(rule.title)}</strong>
                  <span class="hint">/${esc(rule.pattern)}/i · ${rule.type === 'must_contain' ? 'must contain' : 'must not contain'} · ${rule.scope === 'site' ? 'homepage' : 'every page'} · weight ${rule.weight}</span>
                </div>
                <button class="btn ghost small" data-action="remove-rule" data-id="${esc(rule.id)}">Remove</button>
              </div>`,
              )
              .join('')
          : '<p class="hint" style="margin-bottom:12px">No custom checks yet.</p>'
      }
      <div class="form-grid">
        <div class="field full"><label for="cr-title">Rule title</label><input id="cr-title" type="text" placeholder="Licence number is published"></div>
        <div class="field">
          <label for="cr-category">Category</label>
          <select id="cr-category">
            ${categories.map((category) => `<option value="${esc(category.key)}">${esc(category.label)}</option>`).join('')}
          </select>
        </div>
        <div class="field"><label for="cr-weight">Weight (0.5–5)</label><input id="cr-weight" type="number" value="2" min="0.5" max="5" step="0.5"></div>
        <div class="field">
          <label for="cr-type">Type</label>
          <select id="cr-type"><option value="must_contain">Page must contain</option><option value="must_not_contain">Page must not contain</option></select>
        </div>
        <div class="field">
          <label for="cr-scope">Scope</label>
          <select id="cr-scope"><option value="page">Every crawled page</option><option value="site">Homepage only</option></select>
        </div>
        <div class="field full"><label for="cr-pattern">Pattern (regular expression)</label><input id="cr-pattern" type="text" placeholder="licence\\s*(no\\.?|number)?\\s*\\d+" spellcheck="false"></div>
        <div class="field full"><label for="cr-why">Why it matters (shown in reports)</label><input id="cr-why" type="text" placeholder="Licence details are a trust signal for regulated trades."></div>
        <div class="field full"><label for="cr-fix">How to fix</label><input id="cr-fix" type="text" placeholder="Add the licence number to the footer and the About page."></div>
        <div class="field full"><button class="btn secondary" data-action="add-rule">Add custom check</button></div>
      </div>
    </section>

    <section class="card section-card" data-help="standards_guidance">
      <div class="section-head"><h3>Model guidance</h3><span class="hint">injected into every prompt</span></div>
      <p class="section-sub">Tell the models what kind of business this is and which constraints matter. Keep it factual — they will repeat it.</p>
      <div class="field">
        <label for="ai-guidance">Extra context for local models</label>
        <textarea id="ai-guidance" rows="4">${esc(overrides.aiGuidance || '')}</textarea>
      </div>
      <div class="form-grid" style="margin-top:10px">
        <div class="field"><label for="src-label">Your source title</label><input id="src-label" type="text" placeholder="Industry licensing guide"></div>
        <div class="field"><label for="src-url">Your source URL</label><input id="src-url" type="text" placeholder="https://example.com/guidance"></div>
        <div class="field" style="justify-content:flex-end"><button class="btn secondary" data-action="add-source">Add source</button></div>
      </div>
      ${
        sources.length
          ? sources
              .map(
                (source) => `<div class="source-row">
                <a href="${esc(source.url)}" target="_blank" rel="noreferrer">${esc(source.label)}</a>
                <button class="btn ghost small" data-action="remove-source" data-url="${esc(source.url)}">Remove</button>
              </div>`,
              )
              .join('')
          : '<p class="hint" style="margin-top:10px">No custom sources. Built-in checks link their own Google references.</p>'
      }
      <div class="export-row" style="margin-top:16px">
        <button class="btn primary" data-action="save-standards">Save standards</button>
        <span class="hint">Saved to <code>data/overrides.json</code> on this machine.</span>
      </div>
    </section>
  `;

  root.querySelectorAll('.rule-group').forEach((group) => {
    const toggles = [...group.querySelectorAll('.rule-toggle')];
    const count = group.querySelector('.rule-count');
    toggles.forEach((input) => {
      input.addEventListener('change', () => {
        input.closest('.rule-row')?.classList.toggle('off', !input.checked);
        if (count) count.textContent = `${toggles.filter((item) => item.checked).length} of ${toggles.length} on`;
      });
    });
  });

  root.querySelectorAll('.weight-control').forEach((control) => {
    const input = control.querySelector('.weight-input');
    const reset = control.querySelector('.weight-default');
    if (!input || !reset) return;
    const sync = () => {
      const changed = Number(input.value) !== Number(input.dataset.default);
      input.classList.toggle('changed', changed);
      reset.textContent = changed ? `reset to ${input.dataset.default}` : `default ${input.dataset.default}`;
    };
    input.addEventListener('input', sync);
    reset.addEventListener('click', () => {
      input.value = input.dataset.default;
      sync();
    });
    sync();
  });

  decorateHelp(root);
}

async function saveStandards() {
  const disabledRules = $$('.rule-toggle')
    .filter((input) => !input.checked)
    .map((input) => input.dataset.id);
  const weights = {};
  $$('.weight-input').forEach((input) => {
    const value = Number(input.value);
    if (Number.isFinite(value) && value !== Number(input.dataset.default)) weights[input.dataset.id] = value;
  });
  const payload = {
    ...state.overrides,
    disabledRules,
    weights,
    aiGuidance: $('#ai-guidance')?.value ?? state.overrides.aiGuidance,
  };
  try {
    state.overrides = await api('/api/overrides', { method: 'PUT', body: JSON.stringify(payload) });
    toast('Standards saved');
    renderStandards();
    if (state.scan) {
      const data = await api(`/api/scan/${state.scan.id}`);
      state.scan = data.scan;
      state.report = data.report;
      renderReport();
    }
  } catch (error) {
    toast(error.message, true);
  }
}

function addCustomRule() {
  const pattern = $('#cr-pattern').value.trim();
  if (!pattern) {
    toast('Enter a regex pattern', true);
    return;
  }
  try {
    new RegExp(pattern, 'i');
  } catch (error) {
    toast(`Invalid regex: ${error.message}`, true);
    return;
  }
  state.overrides.customRules = state.overrides.customRules ?? [];
  state.overrides.customRules.push({
    id: `custom_${Math.random().toString(36).slice(2, 8)}`,
    title: $('#cr-title').value.trim() || 'Custom check',
    category: $('#cr-category').value,
    weight: Number($('#cr-weight').value) || 1,
    why: $('#cr-why').value.trim(),
    fix: $('#cr-fix').value.trim(),
    type: $('#cr-type').value,
    pattern,
    scope: $('#cr-scope').value,
  });
  renderStandards();
  toast('Custom check added — save standards to apply');
}

function addSource() {
  const label = $('#src-label').value.trim();
  const url = $('#src-url').value.trim();
  if (!/^https?:\/\//i.test(url)) {
    toast('Enter a full http(s) source URL', true);
    return;
  }
  state.overrides.sources = state.overrides.sources ?? [];
  state.overrides.sources.push({ label: label || url, url, note: '' });
  renderStandards();
  toast('Source added — save standards to apply');
}

/* ---------- updates ---------- */

function versionNewer(latest, current) {
  const parse = (value) =>
    String(value || '')
      .replace(/^v/i, '')
      .split('.')
      .map((part) => Number.parseInt(part, 10) || 0);
  const left = parse(latest);
  const right = parse(current);
  const length = Math.max(left.length, right.length);
  for (let i = 0; i < length; i += 1) {
    if ((left[i] || 0) !== (right[i] || 0)) return (left[i] || 0) > (right[i] || 0);
  }
  return false;
}

async function loadUpdates() {
  try {
    state.updates = await api('/api/updates');
  } catch {
    state.updates = { appVersion: '', source: '', content: {}, lastCheck: '', lastError: '' };
  }
}

async function loadContent() {
  try {
    const pack = await api('/api/content');
    if (pack?.topics && typeof pack.topics === 'object') Object.assign(HELP_TOPICS, pack.topics);
    if (pack?.guides && typeof pack.guides === 'object') Object.assign(GUIDES, pack.guides);
  } catch {
    // Built-in help remains available if no content pack exists.
  }
}

function renderUpdates() {
  const root = $('#updates-root');
  if (!root) return;
  const update = state.updates || {};
  const check = state.updateCheck;
  const manifest = check?.manifest || {};
  const appRelease = manifest.app;
  const contentRelease = manifest.content;
  const installedVersion = update.content?.version || '';
  const lastCheck = update.lastCheck ? new Date(update.lastCheck).toLocaleString() : 'Not checked yet';
  const appNewer = appRelease?.latest && versionNewer(appRelease.latest, update.appVersion || check?.appVersion || '0.0.0');
  const contentNewer = contentRelease?.version && contentRelease.version !== installedVersion;

  root.innerHTML = `
    <div class="hero">
      <p class="eyebrow">Updates</p>
      <h2>Keep the living parts current.</h2>
      <p class="lede">Refresh help content and the model catalog without rebuilding the app. Updates are data-only and hash-verified. Custom GGUF models live in the Model lab.</p>
    </div>

    <section class="card section-card" data-help="updates_source">
      <div class="section-head"><h3>Update source</h3><span class="hint">HTTPS manifest</span></div>
      <p class="section-sub">Point GeoSignal at a manifest you publish alongside the app, for example a <code>manifest.json</code> in your GitHub repository. The app never runs code from this URL.</p>
      <div class="form-grid">
        <div class="field full">
          <label for="update-source">Manifest URL</label>
          <input id="update-source" type="url" value="${esc(update.source || '')}" placeholder="https://raw.githubusercontent.com/you/geosignal/main/updates/manifest.json" spellcheck="false">
        </div>
        <div class="export-row">
          <button class="btn secondary" data-action="save-update-source">Save source</button>
          <button class="btn primary" data-action="check-updates">Check for updates</button>
          <span class="hint">${esc(lastCheck)}</span>
        </div>
      </div>
      ${update.lastError ? `<p class="status error" style="margin-top:10px">${esc(update.lastError)}</p>` : ''}
    </section>

    <div class="update-grid">
      <section class="card section-card" data-help="updates_content">
        <div class="section-head"><h3>Help content</h3><span class="chip ${installedVersion ? 'good' : 'quiet'}">${installedVersion ? `updated ${esc(installedVersion)}` : 'built-in'}</span></div>
        <p class="section-sub">Explanations, fix guides and model-catalog entries can be refreshed without a new app binary.</p>
        ${
          contentRelease
            ? `<div class="update-detail">
                <strong>${esc(contentRelease.version || 'New content pack')}</strong>
                <p>${esc(contentRelease.notes || 'No release notes supplied.')}</p>
                <button class="btn secondary small" data-action="apply-content" ${contentNewer ? '' : 'disabled'}>${contentNewer ? 'Install content update' : 'Already installed'}</button>
              </div>`
            : '<p class="hint">Check the update source to see whether new help content is available.</p>'
        }
        ${installedVersion ? '<button class="btn ghost small" data-action="reset-content" style="margin-top:10px">Restore built-in help</button>' : ''}
      </section>

      <section class="card section-card" data-help="updates_app">
        <div class="section-head"><h3>App release</h3><span class="chip ${appNewer ? 'warn' : 'quiet'}">${appNewer ? 'update available' : 'current'}</span></div>
        <p class="section-sub">GeoSignal checks the release metadata; it does not replace its own signed app bundle. Open the release page to download a new build.</p>
        ${
          appRelease
            ? `<div class="update-detail">
                <strong>Latest: ${esc(appRelease.latest || update.appVersion || '—')}</strong>
                <p>${esc(appRelease.notes || 'No release notes supplied.')}</p>
                ${
                  appNewer && appRelease.releaseUrl
                    ? `<a class="btn secondary small" href="${esc(appRelease.releaseUrl)}" target="_blank" rel="noreferrer">Open release page</a>`
                    : '<span class="hint">No newer release published in this manifest.</span>'
                }
              </div>`
            : `<p class="hint">Current version: ${esc(update.appVersion || '0.1.0')}. Check the update source for release information.</p>`
        }
      </section>
    </div>

    <section class="card section-card" data-help="updates_models">
      <div class="section-head"><h3>Model catalog</h3><span class="hint">built-ins + custom</span></div>
      <p class="section-sub">Content updates can add model definitions, but downloads are always explicit and verified. Use the Model lab to add any GGUF model that is not in the catalog.</p>
      <div class="export-row">
        <button class="btn primary small" data-action="open-lab">Open Model lab</button>
        <span class="hint">${engineModels().length} model definition${engineModels().length === 1 ? '' : 's'} loaded</span>
      </div>
    </section>

    <section class="card section-card">
      <div class="section-head"><h3>What updates can change</h3><span class="hint">data only</span></div>
      <ul class="steps">
        <li><strong>Help and fix guides</strong> — explanatory text, steps, code examples and source links.</li>
        <li><strong>Model catalog</strong> — additional GGUF definitions with an HTTPS URL and pinned SHA-256.</li>
        <li><strong>App release metadata</strong> — a version, notes and a link to the release page.</li>
      </ul>
      <p class="hint">Scores, crawl logic, security rules and the app binary require a new signed build. Updates never execute remote code.</p>
    </section>
  `;
  decorateHelp(root);
}

async function saveUpdateSource() {
  const source = $('#update-source')?.value.trim() || '';
  try {
    state.updates = await api('/api/updates/source', { method: 'PUT', body: JSON.stringify({ source }) });
    state.updateCheck = null;
    renderUpdates();
    toast('Update source saved');
  } catch (error) {
    toast(error.message, true);
  }
}

async function checkUpdates() {
  try {
    state.updateCheck = await api('/api/updates/check', { method: 'POST' });
    state.updates = await api('/api/updates');
    renderUpdates();
    toast('Update check complete');
  } catch (error) {
    await loadUpdates();
    renderUpdates();
    toast(error.message, true);
  }
}

async function applyContentUpdate() {
  const content = state.updateCheck?.manifest?.content;
  if (!content) return;
  try {
    const result = await api('/api/updates/apply', { method: 'POST', body: JSON.stringify(content) });
    await loadContent();
    toast(`Content ${result.version} installed — reloading`);
    setTimeout(() => window.location.reload(), 700);
  } catch (error) {
    toast(error.message, true);
  }
}

async function resetContent() {
  try {
    await api('/api/updates/reset', { method: 'POST' });
    await loadUpdates();
    toast('Built-in help restored — reloading');
    setTimeout(() => window.location.reload(), 700);
  } catch (error) {
    toast(error.message, true);
  }
}

async function addCustomModel() {
  const payload = {
    label: $('#custom-gguf-label')?.value.trim() || '',
    url: $('#custom-gguf-url')?.value.trim() || '',
    sha256: $('#custom-gguf-sha')?.value.trim() || '',
    license: $('#custom-gguf-license')?.value.trim() || '',
    family: $('#custom-gguf-family')?.value.trim() || '',
    notes: $('#custom-gguf-notes')?.value.trim() || '',
    size: $('#custom-gguf-size')?.value.trim() || '',
    bytes: 0,
  };
  if (!payload.label || !payload.url || !payload.sha256) {
    toast('Enter a name, HTTPS URL and SHA-256 first', true);
    return;
  }
  try {
    await api('/api/models/custom', { method: 'POST', body: JSON.stringify(payload) });
    await loadModels();
    renderLab();
    toast('Custom model added — download it when ready');
  } catch (error) {
    toast(error.message, true);
  }
}

async function removeCustomModel(id) {
  if (!window.confirm('Remove this custom model entry and its downloaded file?')) return;
  try {
    await api(`/api/models/custom/${encodeURIComponent(id)}`, { method: 'DELETE' });
    const removed = customGgufModels().find((model) => model.id === id);
    if (removed) state.selected.delete(removed.tag);
    localStorage.setItem('geosignal.models', JSON.stringify([...state.selected]));
    await loadModels();
    renderLab();
    toast('Custom model removed');
  } catch (error) {
    toast(error.message, true);
  }
}

/* ---------- exports ---------- */


function exportReport(format) {
  if (!state.scan) return;
  const compareId = state.compare ? `&compareId=${state.compare.id}` : '';
  const url = `/api/scan/${state.scan.id}/report?format=${format}${compareId}`;
  if (format === 'md') {
    const link = document.createElement('a');
    link.href = url;
    link.download = '';
    document.body.appendChild(link);
    link.click();
    link.remove();
  } else if (isDesktopShell() && window.ipc?.postMessage) {
    window.ipc.postMessage(JSON.stringify({ cmd: 'open-url', url: new URL(url, location.href).href }));
  } else {
    window.open(url, '_blank', 'noopener');
  }
}

/* ---------- events ---------- */

function wireEvents() {
  $$('.tab').forEach((tab) =>
    tab.addEventListener('click', () => {
      switchTab(tab.dataset.tab);
      if (tab.dataset.tab === 'updates') renderUpdates();
    }),
  );
  $('#scan-form').addEventListener('submit', runScan);

  $('#help-toggle')?.addEventListener('click', () => {
    state.helpEnabled = !state.helpEnabled;
    localStorage.setItem('geosignal.help', state.helpEnabled ? 'on' : 'off');
    applyHelpSetting();
  });

  $('#help-modal')?.addEventListener('click', (event) => {
    if (event.target.id === 'help-modal') closeHelpPanel();
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && !$('#help-modal')?.hidden) closeHelpPanel();
  });

  document.addEventListener('click', (event) => {
    const linkEl = event.target.closest('a[href]');
    if (linkEl) {
      const href = linkEl.href || '';
      if (isDesktopShell() && window.ipc?.postMessage && /^(https?:|mailto:|tel:)/i.test(href)) {
        event.preventDefault();
        window.ipc.postMessage(JSON.stringify({ cmd: 'open-url', url: href }));
      }
      return;
    }

    const helpEl = event.target.closest('.help-btn[data-help]');
    if (helpEl) {
      event.preventDefault();
      openTopicHelp(helpEl.dataset.help);
      return;
    }

    if (event.target.closest('[data-copy-guide]')) {
      copyGuideCode();
      return;
    }

    const guideEl = event.target.closest('[data-guide]');
    if (guideEl) {
      event.preventDefault();
      openGuide(guideEl.dataset.guide);
      return;
    }

    if (event.target.closest('[data-help-close]')) {
      closeHelpPanel();
      return;
    }

    const scanEl = event.target.closest('[data-scan]');
    if (scanEl) {
      openScan(scanEl.dataset.scan);
      return;
    }
    const actionEl = event.target.closest('[data-action]');
    if (!actionEl) return;
    const action = actionEl.dataset.action;

    if (action === 'go-scan' || action === 'rescan') {
      switchTab('scan');
      $('#url')?.focus();
    } else if (action === 'open-report') {
      switchTab('report');
    } else if (action === 'toggle-findings') {
      state.showAllFindings = !state.showAllFindings;
      renderReport();
    } else if (action === 'open-lab') {
      switchTab('lab');
      renderLab();
    } else if (action === 'save-update-source') {
      saveUpdateSource();
    } else if (action === 'check-updates') {
      checkUpdates();
    } else if (action === 'apply-content') {
      applyContentUpdate();
    } else if (action === 'reset-content') {
      resetContent();
    } else if (action === 'add-custom-model') {
      addCustomModel();
    } else if (action === 'remove-custom-model') {
      removeCustomModel(actionEl.dataset.id);
    } else if (action === 'export-html') {
      exportReport('html');
    } else if (action === 'export-md') {
      exportReport('md');
    } else if (action === 'run-ai') {
      runCompare(actionEl);
    } else if (action === 'download-model') {
      const id = actionEl.dataset.id;
      actionEl.disabled = true;
      actionEl.textContent = 'Starting…';
      api('/api/engine/download', { method: 'POST', body: JSON.stringify({ id }) })
        .then(() => {
          toast('Download started — it stays on this machine');
          pollEngine();
          renderLab();
        })
        .catch((error) => {
          toast(error.message, true);
          actionEl.disabled = false;
          loadModels().then(renderLab);
        });
    } else if (action === 'add-model') {
      const tag = $('#custom-model')?.value.trim();
      if (!tag) {
        toast('Enter a model tag, e.g. phi4-mini', true);
        return;
      }
      if (!state.customModels.includes(tag)) state.customModels.push(tag);
      state.selected.add(tag);
      localStorage.setItem('geosignal.customModels', JSON.stringify(state.customModels));
      localStorage.setItem('geosignal.models', JSON.stringify([...state.selected]));
      renderPickers();
      renderModelStatus();
      toast(`Added ${tag} to the selection`);
    } else if (action === 'recheck-models') {
      refreshEngine()
        .then(() => loadModels())
        .then(() => {
          renderLab();
          toast('Runtime re-checked');
        });
    } else if (action === 'add-rule') {
      addCustomRule();
    } else if (action === 'remove-rule') {
      state.overrides.customRules = (state.overrides.customRules ?? []).filter((rule) => rule.id !== actionEl.dataset.id);
      renderStandards();
      toast('Custom check removed — save standards to apply');
    } else if (action === 'add-source') {
      addSource();
    } else if (action === 'remove-source') {
      state.overrides.sources = (state.overrides.sources ?? []).filter((source) => source.url !== actionEl.dataset.url);
      renderStandards();
      toast('Source removed — save standards to apply');
    } else if (action === 'save-standards') {
      saveStandards();
    }
  });
}

/* ---------- desktop runtime ---------- */

function mountDesktopRuntime() {
  const config = window.__GEOSIGNAL_DESKTOP__;
  if (!config || $('#runtime-panel')) return;
  const port = config.port;

  const chip = document.createElement('button');
  chip.type = 'button';
  chip.id = 'runtime-chip';
  chip.className = 'chip runtime-chip checking';
  chip.innerHTML = '<span class="runtime-dot"></span><span data-runtime-chip>Server</span>';
  const meta = document.querySelector('.topbar-meta') || document.querySelector('.topbar');
  meta?.prepend(chip);

  const panel = document.createElement('div');
  panel.id = 'runtime-panel';
  panel.className = 'runtime-backdrop';
  panel.hidden = true;
  panel.innerHTML = `<div class="runtime-card" role="dialog" aria-modal="true" aria-labelledby="runtime-title">
    <div class="runtime-head">
      <div><p class="eyebrow">Desktop runtime</p><h3 id="runtime-title">Local server &amp; models</h3></div>
      <button class="icon-btn" data-runtime="close" aria-label="Close">×</button>
    </div>
    <div class="runtime-status checking">
      <span class="runtime-dot" data-runtime-dot></span>
      <div><strong data-runtime-state>Checking…</strong><p class="hint" data-runtime-detail>Port ${esc(port)}</p></div>
    </div>
    <dl class="runtime-grid">
      <div><dt>Models</dt><dd data-runtime-models>Checking…</dd></div>
      <div><dt>Server port</dt><dd>${esc(port)}</dd></div>
      <div><dt>Data folder</dt><dd><code>${esc(config.dataDir)}</code></dd></div>
      <div><dt>Version</dt><dd>${esc(config.version)}</dd></div>
    </dl>
    <p class="hint">The app hosts its own server. Stopping it takes the server offline: the window stays open, but scans and reports cannot run until you start it again. A model download already in progress keeps running inside the app and finishes in the background; downloaded models stay in the data folder.</p>
    <div class="runtime-actions">
      <button class="btn primary small" data-runtime="start">Start server</button>
      <button class="btn secondary small" data-runtime="stop">Stop server</button>
      <button class="btn ghost small" data-runtime="browser">Open in browser</button>
      <button class="btn ghost small" data-runtime="reveal">Reveal data folder</button>
      <button class="btn ghost small" data-runtime="quit">Quit app</button>
    </div>
  </div>`;
  document.body.append(panel);

  const runtime = { up: null, pending: '', models: null };
  const chipLabel = chip.querySelector('[data-runtime-chip]');
  const statusBox = panel.querySelector('.runtime-status');
  const stateLabel = panel.querySelector('[data-runtime-state]');
  const detail = panel.querySelector('[data-runtime-detail]');
  const modelsEl = panel.querySelector('[data-runtime-models]');
  const startBtn = panel.querySelector('[data-runtime="start"]');
  const stopBtn = panel.querySelector('[data-runtime="stop"]');
  const browserBtn = panel.querySelector('[data-runtime="browser"]');

  function paint() {
    const up = runtime.up;
    const tone = runtime.pending ? 'checking' : up === true ? 'up' : up === false ? 'down' : 'checking';
    chip.className = `chip runtime-chip ${tone}`;
    chipLabel.textContent = runtime.pending || (up === true ? 'Server on' : up === false ? 'Server off' : 'Server…');
    statusBox.className = `runtime-status ${tone}`;
    stateLabel.textContent = runtime.pending || (up === true ? 'Server running' : up === false ? 'Server stopped' : 'Checking…');
    detail.textContent = up
      ? `Listening on 127.0.0.1:${port}`
      : 'The window stays open. Start the server to scan sites or run models.';
    modelsEl.textContent = runtime.models
      ? `${runtime.models.ready} of ${runtime.models.total} built-in ready`
      : '—';
    startBtn.disabled = Boolean(runtime.pending) || up === true;
    stopBtn.disabled = Boolean(runtime.pending) || up !== true;
    browserBtn.disabled = up !== true;
  }

  let probeSeq = 0;

  async function probe() {
    const seq = ++probeSeq;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 1800);
    let nextUp = false;
    let nextModels = null;
    try {
      const response = await fetch('/api/health', { cache: 'no-store', signal: controller.signal });
      nextUp = response.ok;
      if (response.ok) {
        const data = await fetch('/api/models', { cache: 'no-store', signal: controller.signal })
          .then((res) => res.json())
          .catch(() => null);
        if (data) {
          const models = data.engine?.models ?? [];
          nextModels = { ready: models.filter((model) => model.installed).length, total: models.length };
        }
      }
    } catch {
      nextUp = false;
    } finally {
      clearTimeout(timer);
      // A newer poll (for example after Start) owns the status now.
      if (seq === probeSeq) {
        runtime.up = nextUp;
        if (nextModels) runtime.models = nextModels;
        runtime.pending = '';
        paint();
      }
    }
  }

  function send(cmd) {
    if (!window.ipc || typeof window.ipc.postMessage !== 'function') {
      toast('Desktop controls are only available in the app window', true);
      return;
    }
    window.ipc.postMessage(JSON.stringify({ cmd }));
  }

  chip.addEventListener('click', () => {
    panel.hidden = false;
    probe();
  });

  panel.addEventListener('click', (event) => {
    if (event.target === panel) {
      panel.hidden = true;
      return;
    }
    const action = event.target.closest('[data-runtime]')?.dataset.runtime;
    if (!action) return;
    if (action === 'close') {
      panel.hidden = true;
    } else if (action === 'start') {
      runtime.pending = 'Starting…';
      paint();
      send('start');
      setTimeout(probe, 900);
      setTimeout(probe, 2600);
    } else if (action === 'stop') {
      runtime.pending = 'Stopping…';
      paint();
      send('stop');
      setTimeout(probe, 500);
      setTimeout(probe, 2200);
    } else if (action === 'browser') {
      send('open-browser');
    } else if (action === 'reveal') {
      send('reveal-data');
    } else if (action === 'quit') {
      send('quit');
    }
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && !panel.hidden) panel.hidden = true;
  });

  paint();
  probe();
  setInterval(probe, 5000);
}

window.mountDesktopRuntime = mountDesktopRuntime;

/* ---------- boot ---------- */

async function init() {
  wireEvents();
  applyHelpSetting();
  decorateHelp(document);
  mountDesktopRuntime();
  await loadModels();
  await Promise.allSettled([
    (async () => {
      try {
        state.standards = await api('/api/standards');
      } catch {
        state.standards = { rules: [], categories: [] };
      }
    })(),
    (async () => {
      try {
        state.overrides = { ...DEFAULT_OVERRIDES, ...(await api('/api/overrides')) };
      } catch {
        state.overrides = { ...DEFAULT_OVERRIDES };
      }
    })(),
    loadScans(),
    loadUpdates(),
    loadContent(),
    (async () => {
      try {
        const data = await api('/api/leaderboard');
        state.leaderboard = data.leaderboard ?? [];
      } catch {
        state.leaderboard = [];
      }
    })(),
  ]);

  renderPickers();
  renderRecent();
  renderLab();
  renderStandards();
  renderUpdates();
  renderReport();

  const hash = location.hash.replace('#', '');
  if (['scan', 'report', 'lab', 'standards', 'updates'].includes(hash)) switchTab(hash);
}

init();
