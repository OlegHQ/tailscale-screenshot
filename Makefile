SHELL := /bin/bash

PORT        ?= 7777
TARGET      ?= aarch64-unknown-linux-musl
SERVICE     := tailscale-screenshot
INSTALL_DIR ?= $(HOME)/.local/bin
DATA_DIR    ?= $(HOME)/.local/share/tailscale-screenshot
RELEASE_BIN := server/target/$(TARGET)/release/$(SERVICE)

EXT_DIR     := extension

.PHONY: all build dev install uninstall start stop restart status logs url ps clean help \
        ext-build ext-build-chrome ext-build-firefox \
        ext-unpacked ext-unpacked-chrome ext-unpacked-firefox \
        ext-watch ext-watch-xpi \
        ext-package ext-package-chrome ext-package-firefox ext-clean

all: build

help:
	@echo "Server targets:"
	@echo "  build      — build release binary ($(TARGET))"
	@echo "  dev        — cargo run for local testing (host target)"
	@echo "  install    — build + deploy as smdctl service (user-mode)"
	@echo "  uninstall  — stop and remove the smdctl service"
	@echo "  start      — start the service"
	@echo "  stop       — stop the service"
	@echo "  restart    — restart the service"
	@echo "  status     — show service status"
	@echo "  logs       — follow service logs"
	@echo "  url        — print the server URL the service reported on startup"
	@echo "  clean      — cargo clean"
	@echo ""
	@echo "Extension targets:"
	@echo "  ext-build          — unpacked dirs + packages: .xpi (firefox) and .zip (chrome)"
	@echo "  ext-build-firefox  — unpacked firefox + $(EXT_DIR)/dist/tailscale-screenshot.xpi"
	@echo "  ext-build-chrome   — unpacked chrome + $(EXT_DIR)/dist/tailscale-screenshot-chrome.zip"
	@echo "  ext-unpacked       — only the unpacked dirs (fast; skips web-ext packaging)"
	@echo "  ext-watch          — run in Firefox Developer Edition, live-reload on save"
	@echo "  ext-watch-xpi      — rebuild the .xpi on every source change"
	@echo "  ext-package*       — aliases for ext-build* (packaging is the same step)"
	@echo "  ext-clean          — remove generated extension build/ and dist/"

build:
	@rustup target add $(TARGET) >/dev/null
	cd server && cargo build --release --target $(TARGET)
	@ls -lh $(RELEASE_BIN)

dev:
	cd server && cargo run

install: build
	@mkdir -p $(INSTALL_DIR) $(DATA_DIR)
	install -m 755 $(RELEASE_BIN) $(INSTALL_DIR)/$(SERVICE)
	@sed -e 's|__INSTALL_DIR__|$(INSTALL_DIR)|g' \
	     -e 's|__DATA_DIR__|$(DATA_DIR)|g' \
	     -e 's|__PORT__|$(PORT)|g' \
	     smdctl.yml > .smdctl.rendered.yml
	@if smdctl ps -a 2>/dev/null | grep -q '\b$(SERVICE)\b'; then \
	   smdctl rm -f $(SERVICE) >/dev/null 2>&1 || true; \
	 fi
	smdctl run -f .smdctl.rendered.yml
	@rm -f .smdctl.rendered.yml
	@sleep 1
	@echo ""
	@echo "Service installed. Server URL (configure this in the extension):"
	@$(MAKE) -s url || echo "  (not yet in logs — try: make logs)"
	@echo ""
	@echo "Tip: user-mode services stop on logout unless lingering is enabled:"
	@echo "  loginctl enable-linger $$USER"

uninstall:
	-smdctl rm -f $(SERVICE)
	-rm -f $(INSTALL_DIR)/$(SERVICE)

start:
	smdctl start $(SERVICE)

stop:
	smdctl stop $(SERVICE)

restart:
	smdctl restart $(SERVICE)

status:
	smdctl status $(SERVICE)

logs:
	smdctl logs -f $(SERVICE)

url:
	@smdctl logs -n 200 $(SERVICE) 2>/dev/null | grep -m1 "^Server URL:" | sed 's/^Server URL: //'

ps:
	smdctl ps

clean:
	cd server && cargo clean
	rm -f .smdctl.rendered.yml

# ---- Extension (cross-browser) -------------------------------------------
# A "build" produces both the unpacked dir (build/<browser>/, for load-unpacked
# and web-ext) and the installable package (dist/*.xpi, *-chrome.zip). The
# package.sh step assembles the unpacked dir first, so both fall out together.

ext-build: ext-build-chrome ext-build-firefox

ext-build-chrome:
	$(EXT_DIR)/scripts/package.sh chrome

ext-build-firefox:
	$(EXT_DIR)/scripts/package.sh firefox

# Assemble only the unpacked dirs, skipping the (slower) web-ext packaging.
ext-unpacked: ext-unpacked-chrome ext-unpacked-firefox

ext-unpacked-chrome:
	$(EXT_DIR)/scripts/build.sh chrome

ext-unpacked-firefox:
	$(EXT_DIR)/scripts/build.sh firefox

# Launch in Firefox Developer Edition with automatic reload on source changes.
# Override the binary with FIREFOX_BIN=... if it lives elsewhere.
ext-watch:
	$(EXT_DIR)/scripts/dev-firefox.sh

# Regenerate the installable .xpi automatically whenever a source file changes.
ext-watch-xpi:
	node $(EXT_DIR)/scripts/watch-xpi.js

# Aliases for ext-build (packaging is the same step).
ext-package: ext-build
ext-package-firefox: ext-build-firefox
ext-package-chrome: ext-build-chrome

ext-clean:
	rm -rf $(EXT_DIR)/build $(EXT_DIR)/dist $(EXT_DIR)/src/manifest.json $(EXT_DIR)/web-ext-artifacts
