use crate::types::{CompareResult, Report, ScanResult};
use crate::util::escape_html;
use std::fmt::Write;

fn status_counts(report: &Report) -> (usize, usize, usize, usize) {
    let mut pass = 0;
    let mut warn = 0;
    let mut fail = 0;
    let mut info = 0;
    for check in &report.checks {
        match check.status.as_str() {
            "pass" => pass += 1,
            "warn" => warn += 1,
            "fail" => fail += 1,
            _ => info += 1,
        }
    }
    (pass, warn, fail, info)
}

fn source_links_html(sources: &[crate::types::CheckSource]) -> String {
    sources
        .iter()
        .map(|source| {
            format!(
                "<a href=\"{}\">{}</a>",
                escape_html(&source.url),
                escape_html(&source.label)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn date_label(iso: &str) -> String {
    iso.replace('T', " ").trim_end_matches('Z').to_string()
}

pub fn build_markdown(scan: &ScanResult, report: &Report, compare: Option<&CompareResult>) -> String {
    let mut out = String::new();
    let (pass, warn, fail, info) = status_counts(report);

    let _ = writeln!(out, "# Local SEO report — {}", scan.host);
    let _ = writeln!(out);
    let _ = writeln!(out, "**Score:** {}/100 — {}", report.score, report.band);
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", report.summary);
    let _ = writeln!(out);
    let _ = writeln!(out, "- Starting URL: {}", scan.url);
    let _ = writeln!(
        out,
        "- Crawled: {} page(s) in {} ms on {}",
        scan.pages.len(),
        scan.duration_ms,
        date_label(&scan.finished_at)
    );
    let _ = writeln!(
        out,
        "- Target: {}{}",
        if scan.target.keyword.is_empty() { "service not specified" } else { &scan.target.keyword },
        if scan.target.city.is_empty() {
            String::new()
        } else {
            format!(" in {}", scan.target.city)
        }
    );
    let _ = writeln!(out, "- Tested results: {} pass · {} improve · {} fail · {} note", pass, warn, fail, info);
    if let Some(name) = &scan.signals.business.name {
        let nap = &scan.signals.business;
        let _ = writeln!(
            out,
            "- Detected entity: {}{}{}",
            name,
            nap.locality
                .as_ref()
                .map(|locality| format!(", {}", locality))
                .unwrap_or_default(),
            nap.phone
                .as_ref()
                .map(|phone| format!(" · {}", phone))
                .unwrap_or_default()
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "## Tested results by category");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Category | Score | Passing |");
    let _ = writeln!(out, "| --- | --- | --- |");
    for category in &report.categories {
        let _ = writeln!(
            out,
            "| {} | {}% | {}/{} |",
            category.label, category.score, category.passed, category.total
        );
    }
    let _ = writeln!(out);

    let priority: Vec<_> = report.fixes.iter().take(8).collect();
    let _ = writeln!(out, "## Priority findings");
    let _ = writeln!(out);
    for (index, check) in priority.iter().enumerate() {
        let _ = writeln!(
            out,
            "### {}. {} — {}",
            index + 1,
            check.title,
            if check.status == "fail" { "Fail" } else { "Improve" }
        );
        let _ = writeln!(out, "**Evidence:** {}", check.evidence);
        let _ = writeln!(out);
        let _ = writeln!(out, "**Fix:** {}", check.fix);
        let _ = writeln!(out);
        let _ = writeln!(out, "**Why it matters:** {}", check.why);
        if !check.sources.is_empty() {
            let links = check
                .sources
                .iter()
                .map(|source| format!("[{}]({})", source.label, source.url))
                .collect::<Vec<_>>()
                .join(" · ");
            let _ = writeln!(out, "**Source:** {}", links);
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## What is working");
    let _ = writeln!(out);
    for check in &report.working {
        let _ = writeln!(out, "- **{}** — {}", check.title, check.evidence);
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Crawled pages");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Page | Depth | Status | Words | Links | Indexable |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
    for page in &scan.pages {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            page.path.replace('|', "\\|"),
            page.depth,
            page.status,
            page.word_count,
            page.links.internal,
            if page.noindex { "no" } else { "yes" }
        );
    }
    let _ = writeln!(out);

    if let Some(compare) = compare {
        let _ = writeln!(out, "## Model cross-check");
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "Mode: {} · {} · {}",
            compare.mode,
            compare.runs.iter().map(|run| run.label.clone()).collect::<Vec<_>>().join(", "),
            date_label(&compare.at)
        );
        let _ = writeln!(out);
        let _ = writeln!(out, "| Model | Valid JSON | Latency | Suggestions | Agreement | Score |");
        let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
        for metric in &compare.metrics {
            let _ = writeln!(
                out,
                "| {} | {} | {} ms | {} | {}% | {} |",
                metric.label,
                if metric.valid_json { "yes" } else { "no" },
                metric.latency_ms,
                metric.suggestions,
                metric.consensus_match,
                metric.score
            );
        }
        let _ = writeln!(out);
        for item in compare.curated.iter().take(5) {
            let _ = writeln!(
                out,
                "### {} — agreed by {}/{} models",
                item.title,
                item.vote_count,
                compare.runs.len()
            );
            let _ = writeln!(out, "{}", item.evidence);
            let _ = writeln!(out);
            let _ = writeln!(out, "**Fix:** {}", item.fix);
            let _ = writeln!(out);
            let _ = writeln!(out, "**Why it works:** {}", item.why);
            let _ = writeln!(out);
        }
    }

    let _ = writeln!(out, "## Method and limits");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "This report scores documented local-search signals from a live crawl. It is not a Google ranking position and does not measure traffic. Ranking factors referenced: relevance, distance and prominence (Google Business Profile guidance), title/snippet guidance, LocalBusiness structured data, Core Web Vitals, mobile-first indexing and people-first content guidance. Core Web Vitals field data (LCP <= 2.5 s, INP <= 200 ms, CLS <= 0.1 at p75) must be confirmed in Search Console or PageSpeed Insights."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "Generated locally by GeoSignal.");
    out
}

pub fn build_html_report(scan: &ScanResult, report: &Report, compare: Option<&CompareResult>) -> String {
    let (pass, warn, fail, info) = status_counts(report);
    let categories = report
        .categories
        .iter()
        .map(|category| {
            format!(
                "<div class=\"cat\"><div class=\"cat-head\"><span>{}</span><strong>{}%</strong></div><div class=\"bar\"><i style=\"width:{}%\"></i></div><p class=\"muted\">{} of {} checks</p></div>",
                escape_html(&category.label),
                category.score,
                category.score,
                category.passed,
                category.total
            )
        })
        .collect::<String>();

    let finding = |check: &crate::types::CheckResult| -> String {
        format!(
            "<article class=\"finding {}\">
              <header><span class=\"pill {}\">{}</span><h3>{}</h3></header>
              <p><strong>Evidence.</strong> {}</p>
              <p><strong>Fix.</strong> {}</p>
              <p class=\"why\"><strong>Why.</strong> {}</p>
              <p class=\"src\">{}</p>
            </article>",
            escape_html(&check.status),
            escape_html(&check.status),
            match check.status.as_str() {
                "fail" => "Fail",
                "warn" => "Improve",
                "pass" => "Pass",
                _ => "Note",
            },
            escape_html(&check.title),
            escape_html(&check.evidence),
            escape_html(&check.fix),
            escape_html(&check.why),
            source_links_html(&check.sources)
        )
    };

    let priority = report.fixes.iter().take(8).map(finding).collect::<String>();

    let all_rows = report
        .checks
        .iter()
        .map(|check| {
            format!(
                "<tr><td><span class=\"dot {}\"></span></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&check.status),
                escape_html(&check.title),
                escape_html(&check.evidence),
                match check.status.as_str() {
                    "fail" => "Fail",
                    "warn" => "Improve",
                    "pass" => "Pass",
                    _ => "Note",
                }
            )
        })
        .collect::<String>();

    let passes = report
        .working
        .iter()
        .map(|check| {
            format!(
                "<span class=\"chip\" title=\"{}\">{}</span>",
                escape_html(&check.why),
                escape_html(&check.title)
            )
        })
        .collect::<String>();

    let pages = scan
        .pages
        .iter()
        .map(|page| {
            format!(
                "<tr><td class=\"path\">{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&page.path),
                page.depth,
                page.status,
                page.word_count,
                if page.noindex { "<span class=\"bad\">noindex</span>" } else { "yes" }
            )
        })
        .collect::<String>();

    let entity = {
        let nap = &scan.signals.business;
        let rows = [
            ("Name", nap.name.clone().unwrap_or_else(|| "Not detected".to_string())),
            ("Type", if nap.types.is_empty() { "No LocalBusiness markup".to_string() } else { nap.types.join(", ") }),
            (
                "Address",
                [nap.street.clone(), nap.locality.clone(), nap.region.clone()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            ("Phone", nap.phone.clone().unwrap_or_else(|| "Not in markup".to_string())),
            (
                "Areas served",
                if nap.area_served.is_empty() {
                    "Not set".to_string()
                } else {
                    nap.area_served.join(", ")
                },
            ),
            (
                "Sitemap",
                if scan.signals.sitemap_found {
                    format!("{} URLs", scan.signals.sitemap_urls)
                } else {
                    "Not found".to_string()
                },
            ),
        ];
        rows.iter()
            .map(|(label, value)| {
                format!(
                    "<div class=\"kv\"><span>{}</span><strong>{}</strong></div>",
                    escape_html(label),
                    escape_html(value)
                )
            })
            .collect::<String>()
    };

    let ai = compare.map(|compare| {
        let metrics = compare
            .metrics
            .iter()
            .map(|metric| {
                format!(
                    "<tr><td><strong>{}</strong></td><td>{}</td><td>{:.1}s</td><td>{}</td><td>{}%</td><td><strong>{}</strong></td></tr>",
                    escape_html(&metric.label),
                    if metric.valid_json { "yes" } else { "no" },
                    metric.latency_ms as f64 / 1000.0,
                    metric.suggestions,
                    metric.consensus_match,
                    metric.score
                )
            })
            .collect::<String>();
        let items = compare
            .curated
            .iter()
            .take(5)
            .map(|item| {
                format!(
                    "<article class=\"finding\"><header><span class=\"pill warn\">{} of {} models</span><h3>{}</h3></header><p>{}</p><p><strong>Fix.</strong> {}</p><p class=\"why\"><strong>Why.</strong> {}</p></article>",
                    item.vote_count,
                    compare.runs.len(),
                    escape_html(&item.title),
                    escape_html(&item.evidence),
                    escape_html(&item.fix),
                    escape_html(&item.why)
                )
            })
            .collect::<String>();
        format!(
            "<section><h2>Model cross-check</h2><p class=\"muted\">{} run · {}</p>
             <table><thead><tr><th>Model</th><th>Valid JSON</th><th>Latency</th><th>Suggestions</th><th>Agreement</th><th>Score</th></tr></thead><tbody>{}</tbody></table>
             {}</section>",
            escape_html(&compare.mode),
            escape_html(&compare.runs.iter().map(|run| run.label.clone()).collect::<Vec<_>>().join(", ")),
            metrics,
            items
        )
    }).unwrap_or_default();

    let target = format!(
        "{}{}",
        if scan.target.keyword.is_empty() { "Service not set".to_string() } else { scan.target.keyword.clone() },
        if scan.target.city.is_empty() {
            String::new()
        } else {
            format!(" · {}", scan.target.city)
        }
    );

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Local SEO report — {host}</title>
<style>
  :root {{ --ink:#101828; --muted:#5b6577; --line:#e4e8ef; --brand:#0f766e; --good:#15803d; --warn:#b45309; --bad:#b91c1c; }}
  * {{ box-sizing:border-box; }}
  body {{ margin:0; font:15px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI",Inter,system-ui,sans-serif; color:var(--ink); background:#f6f7f9; }}
  main {{ max-width:920px; margin:0 auto; padding:40px 24px 80px; }}
  header.top {{ display:flex; justify-content:space-between; gap:24px; flex-wrap:wrap; background:#fff; border:1px solid var(--line); border-radius:16px; padding:26px; }}
  h1 {{ font-size:25px; margin:0 0 4px; letter-spacing:-0.02em; }}
  h2 {{ font-size:19px; margin:36px 0 12px; }}
  h3 {{ font-size:15px; margin:0; }}
  .muted {{ color:var(--muted); font-size:13px; }}
  .score {{ font-size:52px; font-weight:700; color:var(--brand); line-height:1; letter-spacing:-0.04em; }}
  .score small {{ font-size:16px; color:var(--muted); font-weight:500; }}
  .band {{ display:inline-block; margin-top:8px; padding:4px 10px; border-radius:999px; background:#e6f4f1; color:var(--brand); font-size:12px; font-weight:700; }}
  .stats {{ display:flex; gap:8px; flex-wrap:wrap; margin-top:14px; }}
  .stat {{ border:1px solid var(--line); border-radius:10px; padding:8px 12px; background:#fff; font-size:12px; color:var(--muted); }}
  .stat strong {{ display:block; font-size:18px; color:var(--ink); }}
  .cats {{ display:grid; grid-template-columns:repeat(auto-fit,minmax(200px,1fr)); gap:12px; margin-top:18px; }}
  .cat {{ background:#fff; border:1px solid var(--line); border-radius:12px; padding:14px; }}
  .cat-head {{ display:flex; justify-content:space-between; font-size:13px; font-weight:600; }}
  .bar {{ height:7px; background:#eef1f5; border-radius:999px; overflow:hidden; margin:8px 0 6px; }}
  .bar i {{ display:block; height:100%; background:linear-gradient(90deg,#0f766e,#2dd4bf); }}
  .finding {{ background:#fff; border:1px solid var(--line); border-left:3px solid var(--warn); border-radius:10px; padding:14px 16px; margin:10px 0; }}
  .finding.fail {{ border-left-color:var(--bad); }}
  .finding header {{ display:flex; gap:10px; align-items:center; flex-wrap:wrap; margin-bottom:6px; }}
  .finding p {{ margin:5px 0; font-size:13.5px; }}
  .why {{ background:#f8fafc; border-radius:8px; padding:8px 10px; }}
  .src {{ font-size:12px; color:var(--muted); }}
  .pill {{ font-size:10px; font-weight:800; text-transform:uppercase; letter-spacing:0.06em; padding:3px 8px; border-radius:999px; }}
  .pill.pass {{ background:#dcfce7; color:var(--good); }}
  .pill.warn {{ background:#fef3c7; color:var(--warn); }}
  .pill.fail {{ background:#fee2e2; color:var(--bad); }}
  .pill.info {{ background:#e0e7ff; color:#3730a3; }}
  table {{ width:100%; border-collapse:collapse; background:#fff; border:1px solid var(--line); border-radius:12px; overflow:hidden; font-size:13px; }}
  th,td {{ text-align:left; padding:9px 11px; border-bottom:1px solid var(--line); vertical-align:top; }}
  th {{ background:#f8fafc; font-size:11px; text-transform:uppercase; letter-spacing:0.04em; color:var(--muted); }}
  tr:last-child td {{ border-bottom:none; }}
  .dot {{ display:inline-block; width:8px; height:8px; border-radius:50%; background:var(--muted); }}
  .dot.pass {{ background:var(--good); }} .dot.warn {{ background:var(--warn); }} .dot.fail {{ background:var(--bad); }}
  .chips {{ display:flex; gap:6px; flex-wrap:wrap; margin-top:10px; }}
  .chip {{ border:1px solid var(--line); border-radius:999px; padding:4px 10px; font-size:12px; background:#fff; }}
  .kv {{ display:flex; justify-content:space-between; gap:12px; border-bottom:1px dashed var(--line); padding:8px 0; font-size:13px; }}
  .kv span {{ color:var(--muted); }}
  .path {{ font-family:ui-monospace,SFMono-Regular,Menlo,monospace; font-size:11.5px; }}
  .bad {{ color:var(--bad); font-weight:600; }}
  a {{ color:var(--brand); }}
  footer {{ margin-top:44px; padding-top:18px; border-top:1px solid var(--line); font-size:12px; color:var(--muted); }}
  @media print {{ body {{ background:#fff; }} main {{ padding:0; max-width:none; }} .finding,table,.cat,header.top {{ break-inside:avoid; }} }}
</style>
</head>
<body>
<main>
  <header class="top">
    <div>
      <h1>Local SEO report — {host}</h1>
      <p class="muted">{url} · {pages} page(s) · {finished}</p>
      <p class="muted">{target}</p>
      <div class="stats">
        <div class="stat"><strong>{pass}</strong>passing</div>
        <div class="stat"><strong>{warn}</strong>to improve</div>
        <div class="stat"><strong>{fail}</strong>failing</div>
        <div class="stat"><strong>{info}</strong>notes</div>
      </div>
    </div>
    <div><div class="score">{score}<small>/100</small></div><span class="band">{band}</span></div>
  </header>
  <p>{summary}</p>
  <div class="cats">{categories}</div>
  <section>
    <h2>Priority findings</h2>
    <p class="muted">Top {priority_count} of {fix_count}. Every finding includes the evidence, the fix and the mechanism.</p>
    {priority}
  </section>
  <section>
    <h2>All tested checks</h2>
    <table><thead><tr><th></th><th>Check</th><th>Evidence</th><th>Result</th></tr></thead><tbody>{all_rows}</tbody></table>
  </section>
  <section>
    <h2>What is working</h2>
    <div class="chips">{passes}</div>
  </section>
  <section>
    <h2>Local entity</h2>
    {entity}
  </section>
  <section>
    <h2>Pages crawled</h2>
    <table><thead><tr><th>Page</th><th>Depth</th><th>Status</th><th>Words</th><th>Indexable</th></tr></thead><tbody>{pages_rows}</tbody></table>
  </section>
  {ai_section}
  <section>
    <h2>Method and limits</h2>
    <p class="muted">This report scores documented local-search signals from a live crawl. It is not a Google ranking position and does not measure traffic. Field Core Web Vitals (LCP ≤ 2.5 s, INP ≤ 200 ms, CLS ≤ 0.1 at p75) must be confirmed in Search Console or PageSpeed Insights.</p>
  </section>
  <footer>Generated locally by GeoSignal · {finished}</footer>
</main>
</body>
</html>"#,
        host = escape_html(&scan.host),
        url = escape_html(&scan.url),
        pages = scan.pages.len(),
        finished = escape_html(&date_label(&scan.finished_at)),
        target = escape_html(&target),
        pass = pass,
        warn = warn,
        fail = fail,
        info = info,
        score = report.score,
        band = escape_html(&report.band),
        summary = escape_html(&report.summary),
        categories = categories,
        priority_count = report.fixes.len().min(8),
        fix_count = report.fixes.len(),
        priority = priority,
        all_rows = all_rows,
        passes = passes,
        entity = entity,
        pages_rows = pages,
        ai_section = ai,
    )
}
