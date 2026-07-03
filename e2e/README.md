# E2E Tests

End-to-end browser tests for the DVD categorizer, using [Playwright](https://playwright.dev/) with TypeScript.

## Prerequisites

1. **Stack running**: `docker compose up -d` at the repo root. Web UI on `:8080`, API on `:3000`.
2. **Node.js 18+** and **npm** on the test-runner machine.
3. **Ollama models pulled** (only for the `ai` project). Use the in-app `/ai/models` page or run:
   ```bash
   docker compose exec ollama ollama pull phi3.5
   docker compose exec ollama ollama pull nomic-embed-text
   ```

## Setup

```bash
cd e2e
npm install
npx playwright install chromium
```

## Running

From the repo root (run `make e2e-install` first if you haven't already):

```bash
make e2e           # run everything: fast + mock + ai
make e2e-fast      # non-AI tests only (parallel, quick)
make e2e-ai        # tests that hit real Ollama (serial, slow)
make e2e-mock      # tests that mock backend responses (parallel, quick)
make e2e-headed    # run `fast` suite in a visible browser
make e2e-ui        # Playwright interactive UI mode
make e2e-report    # open the last HTML report
```

Or directly inside `e2e/`:

```bash
npm test              # all projects
npm run test:fast     # fast only
npm run test:ai       # ai only
npm run test:mock     # mock only
npm run test:ui       # Playwright UI mode (interactive)
npm run codegen       # record a new test
```

## Test inventory

| File | Project | What it covers |
|---|---|---|
| `smoke.spec.ts` | fast | App shell loads, nav links present, API reachable |
| `movies-list.spec.ts` | fast | `/live` route loads and renders the browse bar |
| `browse.spec.ts` | fast | Recently Added / Recent Releases / Random / Unknown Location buttons load results |
| `location.spec.ts` | fast | Location dropdown populates; Movies by Location completes without error |
| `insert-media.spec.ts` | fast | `/movies/new` CSV form renders and accepts input |
| `model-pull.mock.spec.ts` | mock | Pull Model UI: correct request body, success/error display, button disabled while in-flight |
| `ai-chat.ai.spec.ts` | ai | Chat view renders; sending a message produces an AI reply bubble |
| `ai-search.ai.spec.ts` | ai | Vector/Both mode searches complete; model dropdown change works |

## Project layout

```
e2e/
  playwright.config.ts     # project split (fast / ai / mock)
  tests/
    *.spec.ts              # fast project (default)
    *.ai.spec.ts           # ai project (serial, real Ollama)
    *.mock.spec.ts         # mock project (page.route stubs)
  pages/                   # page objects (LivePage, ChatPage)
  utils/                   # shared helpers (waitForAppReady, expectNoConsoleErrors)
```

## Conventions

- **File naming** drives which project runs the test:
  - `name.spec.ts`      → `fast`
  - `name.ai.spec.ts`   → `ai` (workers: 1, retries: 2)
  - `name.mock.spec.ts` → `mock`
- **AI assertions** are tolerant: assert shape/presence, never exact content.
- **No mocks in `fast` or `ai` projects.** Mocks live only in `*.mock.spec.ts`.
- **DB**: tests run against the dev Postgres container. Mutations are permitted. If data is damaged, re-import from production CSV.

## Configuration

Environment variables:

| Var | Default | Purpose |
|---|---|---|
| `E2E_WEB_URL` | `http://localhost:8080` | Web UI base URL |
| `E2E_API_URL` | `http://localhost:3000` | API base URL |

## Debugging failures

- HTML report: `make e2e-report` (or `npx playwright show-report`).
- Trace files are saved on failure; open via the report UI.
- Screenshots and videos land in `test-results/`.
