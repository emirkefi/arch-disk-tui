PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin

.PHONY: all build release install uninstall clean test

all: release

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

install: release
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/arch-disk-tui $(DESTDIR)$(BINDIR)/arch-disk-tui

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/arch-disk-tui

clean:
	cargo clean
