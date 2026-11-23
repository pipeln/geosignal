// Plain-language explainers and fix guides.
// Everything here is intentionally written for a business owner, not a developer.

export const HELP_TOPICS = {
  scan_form: {
    title: 'What this check does',
    body: 'GeoSignal reads the public pages of one website and scores the things Google documents for local businesses. It does not change your site. Enter the address, optionally add the service and city you care about, and run the check. More pages gives a fuller picture but takes longer.',
  },
  field_url: {
    title: 'Website address',
    body: 'The site you want checked. A domain like example.com is enough. We only crawl pages that belong to that site.',
  },
  field_keyword: {
    title: 'Primary service',
    body: 'The main thing customers search for, for example "emergency plumber". Optional, but it lets GeoSignal check whether that service is expressed clearly on the page.',
  },
  field_city: {
    title: 'City or service area',
    body: 'The place you want to be found in, for example "Austin, TX". Optional. It is used to check whether the location appears in titles, headings and copy.',
  },
  field_pages: {
    title: 'Pages to crawl',
    body: 'How many internal pages to read. The homepage is always included. Ten pages is a good balance for a small business site; thirty takes longer and uses more bandwidth.',
  },
  field_mode: {
    title: 'Model run mode',
    body: 'Parallel asks every selected model at the same time, which is faster but uses more memory. Sequential asks them one after another, which is gentler on an older Mac.',
  },
  field_models: {
    title: 'Local models',
    body: 'Optional small AI models that review the crawl independently. They run on this Mac, not in the cloud. The rulebook score is the source of truth; model answers are a cross-check.',
  },
  quick_result: {
    title: 'Latest check',
    body: 'A short summary of the most recent scan. "Open full report" shows the evidence, the reason each check matters and step-by-step fixes.',
  },
  recent_checks: {
    title: 'Recent checks',
    body: 'Scans saved on this Mac. Click one to reopen its report. Nothing is uploaded to a server.',
  },
  how_works: {
    title: 'How the score works',
    body: 'GeoSignal crawls the site, scores 34 documented local-search checks, then explains every result. Some checks carry more weight than others. The score estimates ranking readiness, not a ranking position.',
  },
  report_score: {
    title: 'The score',
    body: 'A weighted summary of the checks that could be tested. Passing checks add points, warnings count partially and failures count as zero. It is a readiness estimate, not a prediction of where you will rank.',
  },
  report_categories: {
    title: 'Category scores',
    body: 'The five groups Google and customers care about: who you are, what the page says, how location pages connect, technical access, and trust. A low category points to where to spend time first.',
  },
  report_findings: {
    title: 'Findings',
    body: 'The checks that need attention, ordered by severity and weight. Open a row for the evidence. "How to fix this" opens a plain-language walkthrough, including who to call and copyable code where it helps.',
  },
  report_tested: {
    title: 'Passing checks',
    body: 'What is already working. Click a check to read what good looks like, so a future edit does not accidentally remove it.',
  },
  report_entity: {
    title: 'Local entity',
    body: 'What search engines can verify about the business: name, address, phone, service areas and sitemap. These values come from the visible page and its structured data, not from Google Business Profile.',
  },
  entity_name: {
    title: 'Business name',
    body: 'The name found in the page or its structured data. It should match Google Business Profile exactly, including punctuation, so Google can connect the site to the right business.',
  },
  entity_schema: {
    title: 'Structured data type',
    body: 'The kind of LocalBusiness markup found, such as Plumber or HomeAndConstructionBusiness. It labels the business for search engines. No markup means Google has to infer everything from visible text.',
  },
  entity_address: {
    title: 'Address',
    body: 'The street address found in the page or its structured data. It should be the same address customers see on Google Business Profile. Service-area businesses can show their area instead of a home address.',
  },
  entity_phone: {
    title: 'Phone',
    body: 'The phone number found in the page or its structured data. Use one main number that matches Google Business Profile and make it tappable on mobile.',
  },
  entity_areas: {
    title: 'Areas served',
    body: 'The cities or regions the business says it serves. They should be real service areas, listed in both visible copy and structured data.',
  },
  entity_sitemap: {
    title: 'Sitemap',
    body: 'The XML sitemap GeoSignal found. It lists pages for search engines. If it is missing, generate one in your CMS or SEO plugin and submit it in Search Console.',
  },
  report_ai: {
    title: 'Model cross-check',
    body: 'Optional AI review of the crawl. Models are grouped by agreement, so a point only one model raised is shown separately. Treat it as a second opinion, not a score.',
  },
  report_pages: {
    title: 'Crawled pages',
    body: 'The pages GeoSignal actually read, with status, word count, links and whether each page asks search engines not to index it.',
  },
  report_method: {
    title: 'Method and limits',
    body: 'What the crawl can and cannot see. Rankings, impressions, clicks and Business Profile performance are not measured here; Search Console and Business Profile Insights hold those numbers.',
  },
  lab_models: {
    title: 'Built-in models',
    body: 'Small language models stored on this Mac. Download once, then they work offline. They are used only to review audit results, not to browse the web.',
  },
  lab_model: {
    title: 'Model card',
    body: 'Each card shows whether the model is ready, where its file is stored and its licence. A 1–3B model is usually enough for this task and fits in normal memory.',
  },
  lab_compare: {
    title: 'Comparison',
    body: 'Choose one or more ready models and run them against the loaded scan. Parallel is faster; sequential uses less memory. Results are validated and compared, not averaged blindly.',
  },
  lab_leaderboard: {
    title: 'Leaderboard',
    body: 'Running totals for each model: how often it returned valid JSON, how fast it was, how complete its answers were and how often it agreed with the other models.',
  },
  lab_latest: {
    title: 'Latest curated run',
    body: 'The most recent model review, grouped by issue and ranked by agreement. Open a note to see the model’s suggested fix and reasoning.',
  },
  lab_custom_models: {
    title: 'Custom GGUF models',
    body: 'Any GGUF model can be added, not just the three built-in presets. You supply an HTTPS download URL and the SHA-256 of the exact file. GeoSignal keeps the entry on this Mac, checks the hash after download, and refuses a file that does not match. The model’s licence stays with its publisher.',
  },
  lab_custom_model: {
    title: 'A custom model card',
    body: 'The card shows whether the model is ready, its size and its licence. Download it before selecting it for a comparison. Removing a custom model deletes its entry and the downloaded file, never the built-in models.',
  },
  updates_source: {
    title: 'Update source',
    body: 'An HTTPS manifest that lists the current help-content pack and app release. It is a data file, not an installer. GeoSignal only reads it when you press Check for updates, and it never executes content from the URL.',
  },
  updates_content: {
    title: 'Help content updates',
    body: 'A content pack can replace or add explanations, fix guides and model definitions. Every pack is checked against the SHA-256 in the manifest before it is written. If the hash does not match, nothing changes.',
  },
  updates_app: {
    title: 'App release checks',
    body: 'GeoSignal can tell you that a newer release exists and open its release page. It does not replace its own signed macOS app bundle automatically. Download and install a new build the same way you installed this one.',
  },
  updates_models: {
    title: 'Model catalog updates',
    body: 'A content pack can add model definitions to the catalog. A model still has to be downloaded explicitly and pass its pinned SHA-256 check. For a model that is not in any catalog, add it under Model lab → Custom GGUF models.',
  },
  standards_checks: {
    title: 'Checks and weights',
    body: 'The full rulebook. Turn a check off to remove it from scoring, or change its weight to say how much it matters for this business. Every row has a fix guide.',
  },
  standards_custom: {
    title: 'Custom checks',
    body: 'Add a regex rule for a local requirement, such as a licence number or a phrase you must never publish. A custom check runs on the crawled pages and appears in future reports.',
  },
  standards_guidance: {
    title: 'Model guidance',
    body: 'Extra context sent to the local models, for example "this is a family-run plumbing business in Austin; never suggest ranking guarantees." Keep it factual and short.',
  },
};

const DEV = 'Your web developer, or whoever built and maintains the site. Ask them to make the change on a staging copy first, then publish it.';

export const GUIDES = {
  https: {
    plain: 'HTTPS is the padlock in the address bar. Without it, browsers warn visitors that the site is "not secure", and people often leave before reading anything.',
    steps: [
      'Open the site and check whether the address starts with https:// and shows a padlock.',
      'Ask your hosting provider to turn on the free SSL certificate (often called Let’s Encrypt) for the domain.',
      'Ask them to redirect every http:// address to the https:// version with one permanent redirect.',
      'Update internal links, the sitemap and any ads or directory listings that still use http://.',
      'Reload the site in a private window and confirm there is no warning.',
    ],
    code: {
      label: 'Apache redirect example (.htaccess) — a developer can adapt this',
      snippet: 'RewriteEngine On\nRewriteCond %{HTTPS} off\nRewriteRule ^(.*)$ https://%{HTTP_HOST}%{REQUEST_URI} [L,R=301]',
    },
    who: 'Your hosting provider can usually enable the certificate. If the site is on Shopify, Squarespace or Wix, it is normally a single switch in the site settings.',
  },
  indexability: {
    plain: 'A "noindex" instruction tells Google not to put a page in search results. It is useful for private pages and a common accident on a live business page.',
    steps: [
      'Open the page and view its source (right-click → View Page Source).',
      'Search for "noindex". If you find it in a robots meta tag, ask your developer to remove it on pages that should appear in Google.',
      'If you use a staging or "coming soon" plugin, check that it is switched off for the live site.',
      'In Google Search Console, use URL Inspection → Request indexing after the change.',
    ],
    code: {
      label: 'What a page should normally say',
      snippet: '<meta name="robots" content="index, follow">',
    },
    who: DEV,
  },
  canonical: {
    plain: 'A canonical tag tells Google which address is the official version of a page, so two similar addresses do not compete with each other.',
    steps: [
      'Choose one address style: https, and either with or without the trailing slash. Use it everywhere.',
      'Add one canonical link in the page head, pointing to that page’s own address.',
      'Do not point every page at the homepage; each page should normally point to itself.',
      'Ask your developer or SEO plugin to generate the tag automatically if the site has many pages.',
    ],
    code: {
      label: 'Canonical tag for a services page',
      snippet: '<link rel="canonical" href="https://example.com/services/plumbing/">',
    },
    who: DEV,
  },
  title: {
    plain: 'The title is the blue clickable line in Google results. It is the strongest short description of what the page is about and where it applies.',
    steps: [
      'Write one title per page. Lead with the service, then the place, then the business name.',
      'Aim for roughly 30–60 characters so it is not cut off on a phone.',
      'Make every location or service page different.',
      'In most website builders this is the "SEO title" or "page title" field; in custom themes it is the <title> tag.',
    ],
    code: {
      label: 'Title example',
      snippet: '<title>Emergency Plumber in Austin, TX | ABC Plumbing</title>',
    },
    who: 'You can write the words yourself. A developer or your CMS SEO plugin can place them.',
  },
  meta_description: {
    plain: 'The description is the short paragraph under the blue link. It does not directly change rankings, but it persuades someone to click instead of a competitor.',
    steps: [
      'Write 120–160 characters: service, city or area, one piece of proof, and a clear next step.',
      'Make it specific to the page; do not reuse the same description everywhere.',
      'Keep it honest. Google may rewrite it, but a good one is often used as written.',
      'Paste it into the page’s meta description field in your CMS.',
    ],
    code: {
      label: 'Description example',
      snippet: '<meta name="description" content="Licensed emergency plumber in Austin, TX. 24/7 repairs, upfront pricing and a 10-year local track record. Call (512) 555-0123.">',
    },
    who: 'You can write it; your CMS or developer can add the field.',
  },
  h1: {
    plain: 'The H1 is the main headline on the page. It tells a reader and Google what the page is about before they read the smaller text.',
    steps: [
      'Use exactly one H1 on the page, and make it the service or the service plus place.',
      'Do not use the logo or a slogan as the H1.',
      'If your theme uses a styled text block instead, ask your developer to change it to a real heading.',
      'Keep the wording natural; it should read like a headline, not a keyword list.',
    ],
    code: {
      label: 'Heading example',
      snippet: '<h1>Emergency Plumbing in Austin, TX</h1>',
    },
    who: DEV,
  },
  headings: {
    plain: 'Subheadings break a long page into scannable sections. They also help Google understand which parts cover services, areas, prices and questions.',
    steps: [
      'Use H2 for the main sections: services, areas served, pricing, reviews, FAQ.',
      'Use H3 for details inside a section, such as one service or one neighbourhood.',
      'Do not skip from H2 to H4, and do not use headings just to make text bigger.',
      'Read the page on a phone: if it feels like a wall of text, add a subheading.',
    ],
    code: {
      label: 'Simple heading structure',
      snippet: '<h2>Plumbing services</h2>\n  <h3>Emergency leaks</h3>\n  <h3>Water heaters</h3>\n<h2>Areas we serve</h2>',
    },
    who: 'You can usually do this in the page editor. A developer is only needed for a custom theme.',
  },
  lang: {
    plain: 'The language declaration tells browsers, screen readers and search engines what language the page is written in.',
    steps: [
      'Ask your developer to confirm the opening <html> tag has the right language code.',
      'Use "en" for English, "en-AU" for Australian English, "es" for Spanish, and so on.',
      'If the site has separate language versions, each version should declare its own language.',
    ],
    code: {
      label: 'Opening HTML tag',
      snippet: '<html lang="en">',
    },
    who: DEV,
  },
  viewport: {
    plain: 'The viewport setting makes a page fit a phone screen instead of showing a tiny desktop layout that users must pinch and zoom.',
    steps: [
      'Ask your developer to add the viewport meta tag to the page head if it is missing.',
      'Do not disable pinch-to-zoom; that hurts accessibility.',
      'Open the site on a real phone and check that text is readable without zooming.',
    ],
    code: {
      label: 'Viewport meta tag',
      snippet: '<meta name="viewport" content="width=device-width, initial-scale=1">',
    },
    who: DEV,
  },
  robots: {
    plain: 'robots.txt is a small public file that tells search engines which parts of the site they may crawl. A mistake here can hide the whole site.',
    steps: [
      'Visit yourdomain.com/robots.txt in a browser.',
      'Make sure it does not block the pages you want in Google, especially CSS and JavaScript files.',
      'Add a line pointing to your sitemap.',
      'Remove rules left over from staging or a "coming soon" setup.',
    ],
    code: {
      label: 'A safe starting robots.txt',
      snippet: 'User-agent: *\nAllow: /\n\nSitemap: https://example.com/sitemap.xml',
    },
    who: DEV,
  },
  sitemap: {
    plain: 'A sitemap is a machine-readable list of the pages you want Google to know about. It helps new or deeply linked pages get discovered.',
    steps: [
      'Use your CMS or SEO plugin to generate an XML sitemap, usually at /sitemap.xml.',
      'Check that the important service and location pages are listed.',
      'Add the sitemap address to robots.txt and submit it in Google Search Console.',
      'Regenerate it after adding pages.',
    ],
    code: {
      label: 'Minimal sitemap example',
      snippet: '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>https://example.com/</loc></url>\n  <url><loc>https://example.com/services/plumbing/</loc></url>\n</urlset>',
    },
    who: 'Most CMS SEO plugins do this automatically. A developer is needed only for a hand-built site.',
  },
  gsc: {
    plain: 'Google Search Console is Google’s free tool for seeing how your site is indexed, which queries it appears for and what errors Google found.',
    steps: [
      'Go to search.google.com/search-console and sign in with the business Google account.',
      'Add your domain as a property. Domain verification is usually done with a DNS record from your hosting provider.',
      'Alternatively verify a URL prefix with an HTML tag in the site head.',
      'Submit the sitemap and check the Pages report for errors after a few days.',
    ],
    code: {
      label: 'HTML verification tag shape',
      snippet: '<meta name="google-site-verification" content="PASTE_THE_CODE_GOOGLE_GIVES_YOU">',
    },
    who: 'You can start this yourself. Your web person or hosting provider may need to add the DNS record.',
  },
  cwv: {
    plain: 'Core Web Vitals measure how quickly the main content appears, how responsive the page feels and whether elements jump around while loading.',
    steps: [
      'Run the homepage and one service page through PageSpeed Insights.',
      'Compress large photos and serve them at the size they are displayed.',
      'Lazy-load images below the first screen, but not the main hero image.',
      'Remove or delay heavy chat widgets, sliders and tracking scripts that load before the content.',
      'Ask a developer to look at LCP (loading), INP (responsiveness) and CLS (layout shift) if the scores are poor.',
    ],
    code: {
      label: 'Image sizing and lazy loading',
      snippet: '<img src="/images/van.jpg" alt="Service van in Austin" width="800" height="500" loading="lazy">',
    },
    who: DEV,
  },
  hreflang: {
    plain: 'hreflang tells Google which language or region a page is for. If the business serves one area in one language, this check usually does not apply.',
    steps: [
      'If the site has only one language, you can ignore this check.',
      'For each language or region, add a link tag pointing to that version.',
      'The tags must be reciprocal: each version links back to the others.',
      'Add an x-default version for visitors whose language does not match.',
    ],
    code: {
      label: 'Three-language example',
      snippet: '<link rel="alternate" hreflang="en-us" href="https://example.com/">\n<link rel="alternate" hreflang="es-us" href="https://example.com/es/">\n<link rel="alternate" hreflang="x-default" href="https://example.com/">',
    },
    who: DEV,
  },
  localbusiness_schema: {
    plain: 'Structured data is a small block of code that labels your business facts for search engines: name, address, phone, opening hours and service area. It is not visible to visitors.',
    steps: [
      'Ask your developer or SEO plugin to add LocalBusiness JSON-LD to the homepage (and one per location if there are several).',
      'Use the exact name, address and phone that appear on Google Business Profile and in the footer.',
      'Add the website, service areas and opening hours.',
      'Test the page with Google’s Rich Results Test and fix any errors it reports.',
    ],
    code: {
      label: 'LocalBusiness JSON-LD starter — replace the example values',
      snippet: '<script type="application/ld+json">\n{\n  "@context": "https://schema.org",\n  "@type": "Plumber",\n  "name": "ABC Plumbing",\n  "url": "https://example.com/",\n  "telephone": "+1-512-555-0123",\n  "address": {\n    "@type": "PostalAddress",\n    "streetAddress": "123 Main St",\n    "addressLocality": "Austin",\n    "addressRegion": "TX",\n    "postalCode": "78701",\n    "addressCountry": "US"\n  },\n  "areaServed": "Austin, TX"\n}\n</script>',
    },
    who: DEV,
  },
  nap_visible: {
    plain: 'Name, address and phone (often called NAP) should be visible as ordinary text, not hidden in an image. They must match Google Business Profile exactly so Google can connect the website to the real business.',
    steps: [
      'Copy the business name, address and phone exactly as they appear in Google Business Profile.',
      'Put them in the footer so they appear on every page, and repeat them on the contact page.',
      'Use text, not a picture of text. Search engines and screen readers cannot read an image.',
      'Make the phone number a tap-to-call link.',
      'If the business hides its address because it works from home, show the service area and phone instead; consistency matters more than publishing a home address.',
    ],
    code: {
      label: 'Footer example — replace with the real details',
      snippet: '<footer>\n  <p><strong>ABC Plumbing</strong><br>\n  123 Main St, Austin, TX 78701<br>\n  <a href="tel:+15125550123">(512) 555-0123</a></p>\n</footer>',
    },
    who: 'You or your web person can edit the footer. If the address differs from Google Business Profile, update the profile first.',
  },
  tel_link: {
    plain: 'A tappable phone number lets someone call with one touch on a phone instead of copying the number by hand.',
    steps: [
      'Wrap the visible phone number in a tel: link.',
      'Use the international format with no spaces or brackets inside the link, for example tel:+15125550123.',
      'Put it in the header or footer so it is on every page.',
      'Test it on a real phone.',
    ],
    code: {
      label: 'Tap-to-call link',
      snippet: '<a href="tel:+15125550123">(512) 555-0123</a>',
    },
    who: DEV,
  },
  geo: {
    plain: 'Coordinates and service-area names help search engines understand where the business actually works, especially when it travels to customers instead of serving them at a shop.',
    steps: [
      'Add geo coordinates and areaServed to the LocalBusiness structured data.',
      'Use the real base location and the real cities or regions served.',
      'List service areas in visible copy too, not only in code.',
      'Do not invent coordinates or claim areas the business cannot serve.',
    ],
    code: {
      label: 'Geo and service area inside LocalBusiness JSON-LD',
      snippet: '"geo": { "@type": "GeoCoordinates", "latitude": 30.2672, "longitude": -97.7431 },\n"areaServed": [\n  { "@type": "City", "name": "Austin" },\n  { "@type": "City", "name": "Round Rock" }\n]',
    },
    who: DEV,
  },
  hours: {
    plain: 'Opening hours should be identical in the footer, on the contact page, in structured data and on Google Business Profile. Conflicting hours make a business look closed or unreliable.',
    steps: [
      'Choose the real hours, including holiday exceptions.',
      'Update Google Business Profile first, then the website footer and contact page.',
      'Add openingHoursSpecification to the LocalBusiness structured data.',
      'Remove old hours from social profiles and directory listings.',
    ],
    code: {
      label: 'Opening hours in JSON-LD',
      snippet: '"openingHoursSpecification": [\n  {\n    "@type": "OpeningHoursSpecification",\n    "dayOfWeek": ["Monday","Tuesday","Wednesday","Thursday","Friday"],\n    "opens": "08:00",\n    "closes": "17:00"\n  }\n]',
    },
    who: 'You can update Google Business Profile and the footer. A developer can add the structured data.',
  },
  same_as: {
    plain: 'sameAs links tell search engines which social and directory profiles belong to the same business, so they can connect the website to those profiles.',
    steps: [
      'Collect the official profiles: Google Business Profile, Facebook, Instagram, LinkedIn, Yelp, trade directories.',
      'Add only profiles that belong to this business, not competitors or unrelated pages.',
      'Put the URLs in sameAs inside the LocalBusiness structured data.',
      'Keep the business name, address and phone consistent on those profiles.',
    ],
    code: {
      label: 'sameAs example',
      snippet: '"sameAs": [\n  "https://www.facebook.com/abcplumbing",\n  "https://www.instagram.com/abcplumbing",\n  "https://maps.google.com/?cid=YOUR_GOOGLE_BUSINESS_ID"\n]',
    },
    who: DEV,
  },
  city_title: {
    plain: 'When the city appears in the page title, both people and search engines can see that the page is about that place, not just the service in general.',
    steps: [
      'Add the city or service area to the title of the page that targets it.',
      'Keep it readable: "Emergency Plumber in Austin, TX | ABC Plumbing".',
      'Give each location page its own city and its own title; do not reuse one title everywhere.',
      'Do not repeat the city unnaturally or stuff several cities into one title.',
    ],
    code: {
      label: 'City in the title',
      snippet: '<title>Water Heater Repair in Round Rock, TX | ABC Plumbing</title>',
    },
    who: 'You or your CMS editor.',
  },
  city_content: {
    plain: 'Mentioning the city in normal sentences, alongside local details, shows that the page is genuinely about that area rather than a template with the name swapped.',
    steps: [
      'Mention the city or service area in the opening paragraph and at least one subheading.',
      'Add real local detail: neighbourhoods, landmarks, typical local problems, travel time or directions.',
      'Write a separate page for each area you actually serve; do not copy one page and change only the city.',
      'Keep the tone natural and useful to a customer.',
    ],
    code: {
      label: 'Natural local paragraph',
      snippet: '<p>We repair water heaters across Austin, from Hyde Park to South Congress. Same-day visits are usually available before 3pm, and we carry common parts for older homes in the area.</p>',
    },
    who: 'You can write this. A copywriter or local marketing person can help if writing is not your thing.',
  },
  keyword: {
    plain: 'The service check looks for the words customers actually use. If the page calls it "thermal system solutions" and customers search "water heater repair", the connection is weaker.',
    steps: [
      'Write down the main service phrase a customer would type.',
      'Use it naturally in the title, H1 and first paragraph.',
      'Include common synonyms once or twice, not in every sentence.',
      'Read it aloud; if it sounds like a keyword list, rewrite it.',
    ],
    code: {
      label: 'Service phrase in a heading',
      snippet: '<h1>Water heater repair in Austin, TX</h1>',
    },
    who: 'You or your copywriter.',
  },
  location_pages: {
    plain: 'A dedicated page for each city or service area gives Google a specific page to rank for that place and gives customers local information instead of a generic homepage.',
    steps: [
      'Create one page per area the business genuinely serves, for example /plumber-austin/ or /areas/round-rock/.',
      'Give each page unique local content, not a copy with the city swapped.',
      'Link the pages from the main menu or a "Service areas" section in the footer.',
      'Include the city in the page title, H1 and copy, plus local proof such as projects or travel details.',
    ],
    code: {
      label: 'Service-area links',
      snippet: '<nav aria-label="Service areas">\n  <a href="/areas/austin/">Austin</a>\n  <a href="/areas/round-rock/">Round Rock</a>\n  <a href="/areas/cedar-park/">Cedar Park</a>\n</nav>',
    },
    who: 'You can plan the pages; a developer or CMS editor can build them.',
  },
  internal_links: {
    plain: 'Internal links are the roads between pages. A page that no other page links to is hard for visitors and search engines to find.',
    steps: [
      'Link the important service and location pages from the homepage, menu or footer.',
      'Use descriptive link text such as "emergency plumbing in Austin", not "click here".',
      'Check for orphan pages: pages that are only reachable from the sitemap.',
      'Keep links as normal HTML links so they work without JavaScript.',
    ],
    code: {
      label: 'Descriptive internal link',
      snippet: '<a href="/services/emergency-plumbing/">Emergency plumbing in Austin</a>',
    },
    who: DEV,
  },
  anchor_relevance: {
    plain: 'Anchor text is the clickable words in a link. It tells the reader, and Google, what the destination page is about before they open it.',
    steps: [
      'Replace generic anchors like "click here", "read more" and "learn more".',
      'Describe the destination: "water heater repair in Round Rock".',
      'Vary the wording naturally across the site.',
      'Point location links at the matching location page, not always at the homepage.',
    ],
    code: {
      label: 'Before and after',
      snippet: '<!-- before -->\n<a href="/areas/round-rock/">Click here</a>\n\n<!-- after -->\n<a href="/areas/round-rock/">Water heater repair in Round Rock</a>',
    },
    who: DEV,
  },
  breadcrumb: {
    plain: 'Breadcrumbs are the small "Home › Services › Plumbing" path near the top of a page. They help people understand where they are and give Google another way to see the site structure.',
    steps: [
      'Add a breadcrumb trail to service and location pages.',
      'Make each step a link except the current page.',
      'Match the trail to the real URL structure.',
      'Ask a developer to add BreadcrumbList structured data if the theme does not do it.',
    ],
    code: {
      label: 'Breadcrumb links',
      snippet: '<nav aria-label="Breadcrumb">\n  <a href="/">Home</a> ›\n  <a href="/services/">Services</a> ›\n  <span>Plumbing</span>\n</nav>',
    },
    who: DEV,
  },
  crawl_depth: {
    plain: 'Crawl depth counts how many clicks a page is from the homepage. Important pages should be reachable in one or two clicks, not buried five levels down.',
    steps: [
      'List the pages that bring in enquiries: services, areas, contact, pricing.',
      'Link them from the homepage, main menu or footer.',
      'Flatten deep menus where possible.',
      'Re-scan after the change and check the Crawled pages table for depth.',
    ],
    who: 'You can decide the structure; a developer or CMS editor can change the menu.',
  },
  content_depth: {
    plain: 'A page needs enough useful copy to answer a customer’s questions. Thin pages give Google little to work with and give visitors no reason to trust the business.',
    steps: [
      'Aim for roughly 500–900 words on main service and location pages, written for people.',
      'Cover the service, who it is for, what is included, pricing or ranges, the process, areas served and common questions.',
      'Add proof: years in business, licences, photos, reviews, guarantees.',
      'Do not pad with repeated keywords; cut anything that does not help the customer.',
    ],
    code: {
      label: 'Useful page outline',
      snippet: '<h1>Water heater repair in Austin</h1>\n<p>Who we help and what we fix.</p>\n<h2>Signs you need a repair</h2>\n<h2>What a visit includes</h2>\n<h2>Pricing and guarantees</h2>\n<h2>Areas we serve</h2>\n<h2>Frequently asked questions</h2>',
    },
    who: 'You or a copywriter. A developer is only needed for layout.',
  },
  helpful_content: {
    plain: 'Trust pages show there are real, accountable people behind the business. Google’s guidance rewards content with first-hand experience and clear ownership.',
    steps: [
      'Add an About page with real names, roles, photos and years in business.',
      'Publish a contact page with a phone number, address or service area and a response promise.',
      'Show licences, insurance, guarantees and trade memberships where they apply.',
      'Add a few real job examples or case studies with photos you own.',
      'Keep privacy and terms pages current.',
    ],
    who: 'You own this content. A copywriter can help shape it; a developer can add the pages.',
  },
  image_alt: {
    plain: 'Alt text describes an image for people using screen readers and for search engines when an image cannot load. It should describe what the image shows.',
    steps: [
      'Add alt text to meaningful images: photos of work, team, premises, diagrams.',
      'Describe the content in a short sentence, including the service or place only when it is genuinely in the picture.',
      'Use alt="" for purely decorative images so screen readers skip them.',
      'Do not stuff keywords into every alt attribute.',
    ],
    code: {
      label: 'Useful alt text',
      snippet: '<img src="/images/water-heater-repair.jpg" alt="Technician repairing a tankless water heater in an Austin home" width="900" height="600" loading="lazy">',
    },
    who: 'Whoever adds images to the page. Most CMS platforms ask for alt text when you upload.',
  },
  map: {
    plain: 'A map or directions link makes it easy for a customer to find the premises or understand the area served, especially on a phone.',
    steps: [
      'Add a directions link or an embedded Google Map to the contact page.',
      'Use the same address as Google Business Profile.',
      'If the business travels to customers, show a service-area map and explain that there is no public shopfront.',
      'Check the map loads on a phone without blocking the rest of the page.',
    ],
    code: {
      label: 'Directions link',
      snippet: '<a href="https://www.google.com/maps/dir/?api=1&destination=123+Main+St+Austin+TX+78701" target="_blank" rel="noopener">Get directions</a>',
    },
    who: 'You can add the link. A developer can embed a map if you want one on the page.',
  },
  reviews: {
    plain: 'Real reviews and testimonials give customers evidence that other people trusted the business. Invented or copied reviews damage trust and can break Google’s policies.',
    steps: [
      'Ask happy customers for a review on Google Business Profile.',
      'Show two or three short quotes on the site with a first name and date or job type.',
      'Link to the full Google reviews rather than copying a whole review feed.',
      'Never invent reviews or mark up other people’s reviews as your own structured data.',
    ],
    code: {
      label: 'A simple, honest testimonial',
      snippet: '<blockquote>\n  <p>“They found the leak the same afternoon and explained the cost before starting.”</p>\n  <footer>— Sarah M., Austin, March 2026</footer>\n</blockquote>',
    },
    who: 'You can collect and publish these yourself. Use the Business Profile to request reviews.',
  },
  faq: {
    plain: 'An FAQ answers the questions customers ask before they call: price, timing, area, guarantees. It also gives Google useful question-and-answer text.',
    steps: [
      'List the questions you answer on the phone every week.',
      'Answer each in two or three plain sentences.',
      'Add them near the bottom of the relevant service page, not hidden in an image or PDF.',
      'Update the answers when prices or policies change.',
    ],
    code: {
      label: 'FAQ section',
      snippet: '<h2>Frequently asked questions</h2>\n<h3>Do you charge a call-out fee?</h3>\n<p>No. We quote the repair before starting, and the visit is free if you decide not to proceed.</p>',
    },
    who: 'You or your team. A developer can add FAQPage structured data if eligible.',
  },
};
