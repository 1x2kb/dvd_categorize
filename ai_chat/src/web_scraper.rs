//! Web Scraping RAG (Retrieval-Augmented Generation) for movie data enrichment.
//!
//! This module is only compiled when the `internet` feature is enabled.
//! It scrapes approved public websites to fetch factual movie data (cast, director,
//! year, genres, plot summary) which is then injected into the LLM prompt so the
//! model produces accurate results rather than hallucinating from training data.
//!
//! No external API keys are required. All data is fetched via plain HTTP GET
//! requests to publicly available pages listed in `APPROVED_SOURCES`.

use log::{debug, info, warn};
use scraper::{Html, Selector};

/// Maximum characters of scraped text to include per movie in the RAG context.
const MAX_CONTEXT_CHARS: usize = 3000;

// ---------------------------------------------------------------------------
// Approved sources
// ---------------------------------------------------------------------------

/// A known-good site that can be queried for movie data.
///
/// Each entry describes how to build candidate URLs and how to search the site.
/// Sources are tried **in order** — the first that yields content wins.
pub struct ApprovedSource {
    /// Human-readable name shown in logs.
    pub name: &'static str,
    /// Base domain (used for allow-list checks and log output).
    pub domain: &'static str,
    /// Build direct-hit URL candidates from a title slug.
    pub candidates_fn: fn(slug: &str) -> Vec<String>,
    /// Build a search/fallback URL when direct hits miss.
    pub search_url_fn: Option<fn(title: &str) -> String>,
    /// CSS selector for the infobox / structured data table.
    pub infobox_selector: &'static str,
    /// CSS selector for body paragraphs.
    pub paragraph_selector: &'static str,
}

/// All sites the scraper is permitted to contact.
///
/// Add new entries here to expand coverage. Sources are tried top-to-bottom
/// per movie title until one returns usable content.
pub static APPROVED_SOURCES: &[ApprovedSource] = &[
    ApprovedSource {
        name: "Wikipedia (film page)",
        domain: "en.wikipedia.org",
        candidates_fn: |slug| {
            vec![
                format!("https://en.wikipedia.org/wiki/{}_(film)", slug),
                format!("https://en.wikipedia.org/wiki/{}", slug),
            ]
        },
        search_url_fn: Some(|title| {
            reqwest::Url::parse_with_params(
                "https://en.wikipedia.org/w/index.php",
                &[("search", format!("{} film", title).as_str()), ("ns0", "1")],
            )
            .map(|u| u.to_string())
            .unwrap_or_else(|_| {
                format!(
                    "https://en.wikipedia.org/w/index.php?search={}&ns0=1",
                    title.replace(' ', "+")
                )
            })
        }),
        infobox_selector: "table.infobox",
        paragraph_selector: "div.mw-parser-output > p",
    },
    ApprovedSource {
        name: "Wikipedia (TV/miniseries page)",
        domain: "en.wikipedia.org",
        candidates_fn: |slug| {
            vec![
                format!("https://en.wikipedia.org/wiki/{}_(TV_series)", slug),
                format!("https://en.wikipedia.org/wiki/{}_(miniseries)", slug),
            ]
        },
        search_url_fn: None,
        infobox_selector: "table.infobox",
        paragraph_selector: "div.mw-parser-output > p",
    },
    ApprovedSource {
        name: "Wikidata simple (film)",
        domain: "www.wikidata.org",
        candidates_fn: |_slug| vec![],
        search_url_fn: Some(|title| {
            reqwest::Url::parse_with_params(
                "https://www.wikidata.org/w/index.php",
                &[("search", title), ("ns0", "1"), ("ns120", "1")],
            )
            .map(|u| u.to_string())
            .unwrap_or_else(|_| {
                format!(
                    "https://www.wikidata.org/w/index.php?search={}&ns0=1&ns120=1",
                    title.replace(' ', "+")
                )
            })
        }),
        infobox_selector: "table.infobox",
        paragraph_selector: "div.mw-parser-output > p",
    },
];

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Outcome of a single URL fetch attempt.
#[derive(Debug, Clone)]
pub enum FetchOutcome {
    /// Page fetched and useful content extracted.
    Success { chars_extracted: usize },
    /// Page fetched but no usable movie content found.
    NoContent,
    /// HTTP error (non-2xx status).
    HttpError { status: u16 },
    /// Network or parse failure.
    Error { reason: String },
    /// URL was skipped (earlier source already succeeded).
    Skipped,
}

/// A record of one URL that was visited during scraping.
#[derive(Debug, Clone)]
pub struct VisitedUrl {
    pub url: String,
    pub source_name: &'static str,
    pub outcome: FetchOutcome,
}

/// Full scrape report for a single movie title.
#[derive(Debug, Clone)]
pub struct MovieScrapedContext {
    pub title: String,
    /// The extracted factual text injected into the RAG prompt.
    pub context: String,
    /// The URL that ultimately provided the context (empty if nothing found).
    pub source_url: String,
    /// Every URL that was visited while looking for this movie.
    pub visited_urls: Vec<VisitedUrl>,
}

impl MovieScrapedContext {
    /// Log a human-readable scrape report at INFO level.
    pub fn log_report(&self) {
        let found = !self.context.is_empty();
        info!(
            "[RAG scrape] '{}' — {} ({} chars) | source: {}",
            self.title,
            if found { "FOUND" } else { "NOT FOUND" },
            self.context.len(),
            if self.source_url.is_empty() {
                "none"
            } else {
                &self.source_url
            }
        );
        info!(
            "[RAG scrape] '{}' — visited {} URL(s):",
            self.title,
            self.visited_urls.len()
        );
        for v in &self.visited_urls {
            match &v.outcome {
                FetchOutcome::Success { chars_extracted } => info!(
                    "[RAG scrape]   ✓ {} chars  [{}]  {}",
                    chars_extracted, v.source_name, v.url
                ),
                FetchOutcome::NoContent => debug!(
                    "[RAG scrape]   ○ no content  [{}]  {}",
                    v.source_name, v.url
                ),
                FetchOutcome::HttpError { status } => warn!(
                    "[RAG scrape]   ✗ HTTP {}  [{}]  {}",
                    status, v.source_name, v.url
                ),
                FetchOutcome::Error { reason } => warn!(
                    "[RAG scrape]   ✗ error: {}  [{}]  {}",
                    reason, v.source_name, v.url
                ),
                FetchOutcome::Skipped => debug!(
                    "[RAG scrape]   - skipped  [{}]  {}",
                    v.source_name, v.url
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fetch web context for a slice of movie titles concurrently.
///
/// Returns one `MovieScrapedContext` per title (in the same order as input).
/// If scraping fails for a title the `context` field will be empty.
/// Logs a full scrape report for every title when complete.
pub async fn scrape_movie_contexts(titles: &[String]) -> Vec<MovieScrapedContext> {
    let client = build_client();

    info!(
        "[RAG scrape] Starting scrape for {} title(s): [{}]",
        titles.len(),
        titles.join(", ")
    );
    info!(
        "[RAG scrape] Approved sources: {}",
        APPROVED_SOURCES
            .iter()
            .map(|s| s.name)
            .collect::<Vec<_>>()
            .join(" | ")
    );

    let handles: Vec<_> = titles
        .iter()
        .map(|title| {
            let client = client.clone();
            let title = title.clone();
            tokio::spawn(async move { scrape_for_movie(&client, &title).await })
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(ctx) => {
                ctx.log_report();
                results.push(ctx);
            }
            Err(e) => {
                warn!("[RAG scrape] Scrape task panicked: {}", e);
            }
        }
    }

    let found = results.iter().filter(|r| !r.context.is_empty()).count();
    info!(
        "[RAG scrape] Complete — {}/{} titles yielded usable context",
        found,
        results.len()
    );

    results
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build an HTTP client with a browser-like User-Agent.
fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (compatible; MovieDataBot/1.0; +https://github.com/dvd-categorize)",
        )
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("Failed to build reqwest client")
}

/// Try every approved source in order until one returns content.
async fn scrape_for_movie(client: &reqwest::Client, title: &str) -> MovieScrapedContext {
    info!("[RAG scrape] Looking up '{}'", title);

    let slug = title_to_slug(title);
    let mut visited: Vec<VisitedUrl> = Vec::new();

    for source in APPROVED_SOURCES {
        let candidates = (source.candidates_fn)(&slug);

        for url in &candidates {
            debug!("[RAG scrape] Trying {} — {}", source.name, url);
            match fetch_and_extract(client, url, source).await {
                Ok((context, final_url)) => {
                    let chars = context.len();
                    visited.push(VisitedUrl {
                        url: url.clone(),
                        source_name: source.name,
                        outcome: FetchOutcome::Success {
                            chars_extracted: chars,
                        },
                    });
                    mark_remaining_skipped(&candidates, url, source.name, &mut visited);
                    return MovieScrapedContext {
                        title: title.to_string(),
                        context,
                        source_url: final_url,
                        visited_urls: visited,
                    };
                }
                Err(outcome) => {
                    visited.push(VisitedUrl {
                        url: url.clone(),
                        source_name: source.name,
                        outcome,
                    });
                }
            }
        }

        if let Some(search_fn) = source.search_url_fn {
            let search_url = search_fn(title);
            debug!(
                "[RAG scrape] Search fallback {} — {}",
                source.name, search_url
            );
            match fetch_search_page(client, &search_url, title, source).await {
                Some((context, final_url, chars)) => {
                    visited.push(VisitedUrl {
                        url: search_url,
                        source_name: source.name,
                        outcome: FetchOutcome::Success {
                            chars_extracted: chars,
                        },
                    });
                    return MovieScrapedContext {
                        title: title.to_string(),
                        context,
                        source_url: final_url,
                        visited_urls: visited,
                    };
                }
                None => {
                    visited.push(VisitedUrl {
                        url: search_url,
                        source_name: source.name,
                        outcome: FetchOutcome::NoContent,
                    });
                }
            }
        }
    }

    warn!("[RAG scrape] No content found for '{}'", title);
    MovieScrapedContext {
        title: title.to_string(),
        context: String::new(),
        source_url: String::new(),
        visited_urls: visited,
    }
}

/// Fetch a direct URL and extract movie context. Returns `Err(FetchOutcome)` on failure.
async fn fetch_and_extract(
    client: &reqwest::Client,
    url: &str,
    source: &ApprovedSource,
) -> Result<(String, String), FetchOutcome> {
    let resp = client.get(url).send().await.map_err(|e| FetchOutcome::Error {
        reason: e.to_string(),
    })?;

    if !resp.status().is_success() {
        return Err(FetchOutcome::HttpError {
            status: resp.status().as_u16(),
        });
    }

    let final_url = resp.url().to_string();
    let html = resp
        .text()
        .await
        .map_err(|e| FetchOutcome::Error {
            reason: e.to_string(),
        })?;

    let context = extract_context_from_html(&html, source);
    if context.is_empty() {
        return Err(FetchOutcome::NoContent);
    }

    if !context_looks_like_film(&context) {
        debug!(
            "[RAG scrape] Page at {} has content but doesn't look like a film article — skipping",
            final_url
        );
        return Err(FetchOutcome::NoContent);
    }

    debug!(
        "[RAG scrape] Extracted {} chars from {}",
        context.len(),
        final_url
    );
    Ok((context, final_url))
}

/// Heuristic check: does the scraped context look like a film/TV article?
///
/// Looks for at least one film-specific infobox field or keyword in the first
/// paragraph. This filters out disambiguation pages, ship articles, etc.
fn context_looks_like_film(context: &str) -> bool {
    let lower = context.to_lowercase();
    let film_signals = [
        "directed by",
        "starring",
        "produced by",
        "screenplay by",
        "written by",
        "release date",
        "running time",
        "box office",
        "cinematography",
        "distributed by",
        "film directed",
        "television series",
        "miniseries",
        "animated series",
        "is a film",
        "is an american film",
        "is a british film",
        "is a comedy film",
        "is a drama film",
        "is an action film",
        "is a horror film",
        "is a science fiction film",
        "is a documentary film",
    ];
    film_signals.iter().any(|signal| lower.contains(signal))
}

/// Fetch a search/fallback URL, follow redirects, and try the first result if needed.
async fn fetch_search_page(
    client: &reqwest::Client,
    search_url: &str,
    title: &str,
    source: &ApprovedSource,
) -> Option<(String, String, usize)> {
    let resp = client.get(search_url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let final_url = resp.url().to_string();
    let html = resp.text().await.ok()?;

    if final_url.contains("/wiki/") && !final_url.contains("Special:Search") {
        let context = extract_context_from_html(&html, source);
        if !context.is_empty() {
            let chars = context.len();
            debug!(
                "[RAG scrape] Search redirect landed on article: {} ({} chars)",
                final_url, chars
            );
            return Some((context, final_url, chars));
        }
    }

    if let Some(href) = extract_first_search_result_url(&html) {
        let result_url = if href.starts_with("http") {
            href
        } else {
            format!("https://{}{}", source.domain, href)
        };
        debug!(
            "[RAG scrape] Following first search result for '{}': {}",
            title, result_url
        );
        if let Ok((context, resolved_url)) = fetch_and_extract(client, &result_url, source).await {
            let chars = context.len();
            return Some((context, resolved_url, chars));
        }
    }

    None
}

/// Mark candidate URLs that were not tried as skipped (because an earlier one succeeded).
fn mark_remaining_skipped(
    candidates: &[String],
    succeeded: &str,
    source_name: &'static str,
    visited: &mut Vec<VisitedUrl>,
) {
    for url in candidates {
        if url != succeeded {
            visited.push(VisitedUrl {
                url: url.clone(),
                source_name,
                outcome: FetchOutcome::Skipped,
            });
        }
    }
}

/// Extract relevant text from an HTML page using the source's configured selectors.
///
/// Collects:
/// - Infobox / structured-data table rows (cast, director, genres, release date)
/// - First 3 substantive body paragraphs (plot / overview)
fn extract_context_from_html(html: &str, source: &ApprovedSource) -> String {
    let document = Html::parse_document(html);
    let mut parts: Vec<String> = Vec::new();

    if let Ok(infobox_sel) = Selector::parse(source.infobox_selector) {
        let tr_sel = Selector::parse("tr").unwrap();
        let th_sel = Selector::parse("th").unwrap();
        let td_sel = Selector::parse("td").unwrap();

        if let Some(infobox) = document.select(&infobox_sel).next() {
            let mut infobox_lines: Vec<String> = Vec::new();
            for row in infobox.select(&tr_sel) {
                let header: String = row
                    .select(&th_sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_default();
                let value: String = row
                    .select(&td_sel)
                    .next()
                    .map(|el| {
                        el.text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();

                if !header.is_empty() && !value.is_empty() {
                    infobox_lines.push(format!("{}: {}", header, value));
                }
            }
            if !infobox_lines.is_empty() {
                debug!(
                    "[RAG scrape] Infobox: {} fields extracted",
                    infobox_lines.len()
                );
                parts.push(format!("=== Infobox ===\n{}", infobox_lines.join("\n")));
            }
        }
    }

    if let Ok(p_sel) = Selector::parse(source.paragraph_selector) {
        let mut para_count = 0;
        for para in document.select(&p_sel) {
            let text: String = para.text().collect::<String>();
            let cleaned = text.trim().to_string();
            if !cleaned.is_empty() && cleaned.len() > 40 {
                parts.push(cleaned);
                para_count += 1;
                if para_count >= 3 {
                    break;
                }
            }
        }
        debug!("[RAG scrape] Body paragraphs extracted: {}", para_count);
    }

    let combined = parts.join("\n\n");
    if combined.len() > MAX_CONTEXT_CHARS {
        combined[..MAX_CONTEXT_CHARS].to_string()
    } else {
        combined
    }
}

/// Extract the href of the first real article result from a search results page.
fn extract_first_search_result_url(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let result_sel =
        Selector::parse("div.mw-search-result-heading a, ul.mw-search-results li a").ok()?;

    document
        .select(&result_sel)
        .next()
        .and_then(|el| el.value().attr("href"))
        .map(|href| href.to_string())
}

/// Convert a movie title to a URL slug (spaces → underscores).
fn title_to_slug(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .replace('/', "-")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_to_slug() {
        assert_eq!(title_to_slug("The Matrix"), "The_Matrix");
        assert_eq!(title_to_slug("Schindler's List"), "Schindler's_List");
    }

    #[test]
    fn test_extract_context_empty_html() {
        let source = &APPROVED_SOURCES[0];
        let result = extract_context_from_html("<html><body></body></html>", source);
        assert!(result.is_empty());
    }

    #[test]
    fn test_approved_sources_not_empty() {
        assert!(!APPROVED_SOURCES.is_empty());
        for s in APPROVED_SOURCES {
            assert!(!s.name.is_empty());
            assert!(!s.domain.is_empty());
        }
    }

    #[test]
    fn test_visited_url_tracking() {
        let ctx = MovieScrapedContext {
            title: "Test Movie".to_string(),
            context: String::new(),
            source_url: String::new(),
            visited_urls: vec![
                VisitedUrl {
                    url: "https://en.wikipedia.org/wiki/Test_Movie_(film)".to_string(),
                    source_name: "Wikipedia (film page)",
                    outcome: FetchOutcome::HttpError { status: 404 },
                },
                VisitedUrl {
                    url: "https://en.wikipedia.org/wiki/Test_Movie".to_string(),
                    source_name: "Wikipedia (film page)",
                    outcome: FetchOutcome::NoContent,
                },
            ],
        };
        assert_eq!(ctx.visited_urls.len(), 2);
        assert!(ctx.context.is_empty());
    }
}
