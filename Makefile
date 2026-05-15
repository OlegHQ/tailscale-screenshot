SHELL := /bin/bash

PORT        ?= 7777
TARGET      ?= aarch64-unknown-linux-musl
SERVICE     := tailscale-screenshot
INSTALL_DIR ?= $(HOME)/.local/bin
DATA_DIR    ?= $(HOME)/.local/share/tailscale-screenshot
RELEASE_BIN := server/target/$(TARGET)/release/$(SERVICE)

.PHONY: all build dev install uninstall start stop restart status logs url ps clean help

all: build

help:
	@echo "Targets:"
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
