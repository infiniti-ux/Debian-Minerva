# Debian Minerva — developer shortcuts
SHELL := /bin/bash
PORT ?= 8080
PREFIX ?= /usr/local

.PHONY: build run test self-test preview clean install uninstall reference

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
	mkdir -p $(DESTDIR)$(PREFIX)/share/debian-minerva
	cp -r web/. $(DESTDIR)$(PREFIX)/share/debian-minerva/web/
	install -Dm644 packaging/minerva.desktop $(DESTDIR)$(PREFIX)/share/xsessions/minerva.desktop
	install -Dm644 packaging/minerva-session.service $(DESTDIR)$(PREFIX)/lib/systemd/user/minerva-session.service

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-sessiond
	rm -rf $(DESTDIR)$(PREFIX)/share/debian-minerva
	rm -f $(DESTDIR)$(PREFIX)/share/xsessions/minerva.desktop
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/user/minerva-session.service

clean:
	cargo clean
