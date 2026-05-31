# DVD Categorize - Microservice Architecture Plan

## Current State
Monolithic architecture with all services on shared network. API has full internet access.

## Target Architecture
Isolated networks with controlled egress through dedicated microservices.

## Phase 1: Network Isolation (Immediate)

### Networks
```yaml
backend:
  internal: true  # No internet - API, DB, cache, monitoring
internet:
  # Has internet - Ollama, scraper-service
```

### Service Placement
| Service | Network | Internet | Notes |
|---------|---------|----------|-------|
| dvd_catalog_api | backend | ❌ | Main API - isolated |
| postgres | backend | ❌ | Database |
| redis | backend | ❌ | Cache |
| grafana | backend | ❌ | Dashboards |
| prometheus | backend | ❌ | Metrics |
| loki | backend | ❌ | Logs |
| promtail | backend | ❌ | Log shipper |
| cadvisor | backend | ❌ | Container metrics |
| node-exporter | backend | ❌ | Host metrics |
| ollama | backend + internet | ✅ | Needs model downloads |
| scraper-service | backend + internet | ✅ | Web scraping only |

## Phase 2: Scraper Microservice

### Purpose
Controlled egress point for all web scraping operations.

### API
```
POST /scrape
{
  "url": "https://...",
  "selectors": [...],
  "timeout_ms": 30000
}
```

Response:
```
{
  "status": "success|error",
  "data": { ... },
  "scraped_at": "2024-..."
}
```

### Security Features
- URL allowlist (only specific domains)
- Request timeout limits
- Response size limits
- Rate limiting per client
- Audit logging of all requests
- No arbitrary redirects followed

### Implementation
- Tiny Axum service (~200 lines)
- reqwest for HTTP client
- scraper crate for HTML parsing
- Separate Docker image (minimal attack surface)

## Phase 3: Generator Completion

### Current Issue
AI title correction has intermittent failures. Debug logging added, need to:

1. Review logs from Grafana to identify failure patterns
2. Add fallback behavior when AI correction fails
3. Ensure streaming response handles edge cases

### Tasks
- [ ] Review `correct_movie_titles` logs in Grafana
- [ ] Identify timeout vs parsing vs model error patterns
- [ ] Implement exponential backoff for AI calls
- [ ] Add fallback: use original titles if AI fails after retries
- [ ] Add user-facing error messages in Generation UI

## Implementation Order

1. **Create `scraper` crate** - New microservice
2. **Update docker-compose.yml** - Add networks, move services
3. **Update API** - Change scraper calls from direct HTTP to internal service call
4. **Test network isolation** - Verify API cannot reach internet
5. **Finish Generator** - Use new logging to debug and fix

## Testing Network Isolation

```bash
# Should FAIL - API cannot reach internet
docker exec dvd_categorize_api_1 curl https://evil.com

# Should SUCCEED - Ollama can reach internet
docker exec dvd_categorize_ollama_1 curl https://ollama.com

# Should SUCCEED - Scraper can reach allowed sites
docker exec dvd_categorize_scraper_1 curl https://imdb.com

# Should SUCCEED - API can talk to scraper internally
docker exec dvd_categorize_api_1 curl http://scraper-service:3000/health
```
