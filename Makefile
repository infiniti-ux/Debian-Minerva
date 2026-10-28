# Debian Minerva — developer shortcuts
SHELL := /bin/bash
PORT ?= 8080
PREFIX ?= /usr/local

.PHONY: build run test self-test preview clean install uninstall reference deb deb-deps session qa test-shell test-session test-all wm-smoke

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
	install -Dm755 target/release/minerva-wm $(DESTDIR)$(PREFIX)/bin/minerva-wm
	install -Dm755 session/minerva-session $(DESTDIR)$(PREFIX)/bin/minerva-session
	install -Dm755 session/minerva-ctl $(DESTDIR)$(PREFIX)/bin/minerva-ctl
	mkdir -p $(DESTDIR)$(PREFIX)/share/minerva/web
	cp -r web/. $(DESTDIR)$(PREFIX)/share/minerva/web/
	install -Dm644 session/openbox/menu.xml $(DESTDIR)$(PREFIX)/share/minerva/openbox/menu.xml
	install -Dm644 session/minerva.desktop $(DESTDIR)$(PREFIX)/share/xsessions/minerva.desktop
	install -Dm644 packaging/minerva-sessiond.service $(DESTDIR)$(PREFIX)/lib/systemd/user/minerva-sessiond.service

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-sessiond
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-wm
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-session
	rm -f $(DESTDIR)$(PREFIX)/bin/minerva-ctl
	rm -rf $(DESTDIR)$(PREFIX)/share/minerva
	rm -f $(DESTDIR)$(PREFIX)/share/wayland-sessions/minerva.desktop
	rm -f $(DESTDIR)$(PREFIX)/share/xsessions/minerva.desktop
	rm -f $(DESTDIR)$(PREFIX)/lib/systemd/user/minerva-sessiond.service

# Build deps for the .deb — minerva-wm (native WM/compositor) needs the X11
# dev libraries at BUILD time; the runtime libs come in via shlibs.
X11_BUILD_DEPS := libx11-dev libxext-dev libxrender-dev libxcomposite-dev libxdamage-dev libxfixes-dev
DEB_BUILD_DEPS := debhelper cargo rustc $(X11_BUILD_DEPS)

# Install everything `make deb` needs (asks for sudo itself)
deb-deps:
	sudo apt-get install -y $(DEB_BUILD_DEPS)

# Build the installable .deb (then: sudo apt install ../debian-minerva_0.11.0_*.deb)
# Pre-flight: missing build deps become the exact apt command, not a cryptic
# dpkg-checkbuilddeps abort halfway through.
deb:
	@missing=""; \
	for p in $(DEB_BUILD_DEPS); do \
	  dpkg -s "$$p" >/dev/null 2>&1 || missing="$$missing $$p"; \
	done; \
	if [ -n "$$missing" ]; then \
	  echo "make deb: missing build dependencies:$$missing"; \
	  echo ""; \
	  echo "  fix with:  sudo apt install -y$$missing"; \
	  echo "  (or run:   make deb-deps)"; \
	  echo ""; \
	  exit 3; \
	fi
	dpkg-buildpackage -us -uc -b

# Run the full desktop session right here (needs X + openbox; falls back to cage)
session: build
	MINERVA_DAEMON=./target/release/minerva-sessiond MINERVA_WM=./target/release/minerva-wm MINERVA_WEBROOT=./web ./session/minerva-session

wm-smoke: build
	xvfb-run -a -s "-screen 0 1280x1024x24" ./target/release/minerva-wm --smoke

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
