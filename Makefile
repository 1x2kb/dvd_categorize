.PHONY: e2e e2e-install e2e-fast e2e-ai e2e-mock e2e-headed e2e-ui e2e-report

E2E_DIR := e2e

e2e-install:
	cd $(E2E_DIR) && npm install && npx playwright install chromium

e2e: e2e-install
	cd $(E2E_DIR) && npm test

e2e-fast:
	cd $(E2E_DIR) && npm run test:fast

e2e-ai:
	cd $(E2E_DIR) && npm run test:ai

e2e-mock:
	cd $(E2E_DIR) && npm run test:mock

e2e-headed:
	cd $(E2E_DIR) && npm run test:headed

e2e-ui: e2e-install
	cd $(E2E_DIR) && npm run test:ui

e2e-report:
	cd $(E2E_DIR) && npm run report
