# Debian Minerva — developer shortcuts
SHELL := /bin/bash
PORT ?= 8080
PREFIX ?= /usr/local

.PHONY: build run test self-test preview clean install uninstall reference deb session qa test-shell test-session test-all

build:
	cargo build --release

run: build
	./target/release/minerva-sessiond --port $(PORT) --web-root ./web

test:
	cargo test --all

self-test: build
	./target/release/minerva-sessiond --self-test

preview:
	node tools/preview.mjs --port $(PORT)

reference:
	python3 references/aluminium-os/render.py

install: build
	install -Dm755 target/release/minerva-sessiond $(DESTDIR)$(PREFIX)/bin/minerva-sessiond
	install -Dm755 session/minerva-session $(DESTDIR)$(PREFIX)/bin/minerva-session
	install -Dm755 session/minerva-ctl $(DESTDIR)$(PREFIX)/bin/minerva-ctl
	mkdir -p $(DESTDIR)$(PREFIX)/share/minerva/web
	cp -r web/. $(DESTDIR)$(PREFIX)/share/minerva/web/
	install -Dm644 session/openbox/menu.xml $(DESTDIR)$(PREFIX)/share/minerva/openbox/menu.xml
	install -Dm644 session/minerva.desktop $(DESTDIR)$(PREFIX)/share/xsessions/minerva.desktop
	install -Dm644 packaging/minerva-sessiond.service $(DESTDIR)$(PREFIX)/lib/systemd/user/minerva-sessiond.service

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-sessiond
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-session
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-ctl
	rm -rf $(DESTDIR)$(PREFIX)/share/minerva
	rm -f $(DESTDIR)$(PREFIX)/share/wayland-sessions/minerva.desktop
	rm -f $(DESTDIR)$(PREFIX)/share/xsessions/minerva.desktop
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/user/minerva-sessiond.service

# Build the installable .deb (then: sudo apt install ../debian-minerva_0.2.0_*.deb)
deb:
	dpkg-buildpackage -us -uc -b

# Run the full desktop session right here (needs X + openbox; falls back to cage)
session: build
	MINERVA_DAEMON=./target/release/minerva-sessiond MINERVA_WEBROOT=./web ./session/minerva-session

clean:
	cargo clean

qa:
	./tools/qa.sh

test-shell:
	./tools/shell-test/run.sh

test-session:
	./tools/session-test/run.sh

test-all: qa test-shell test-session
	@echo "test-all: complete"
