# chan public build surface.
#
# Keep this file as the command contract. Platform and package details belong
# in subdirectories such as desktop/ and packaging/linux/.

.DEFAULT_GOAL := help

PREFIX ?= $(if $(XDG_BIN_HOME),$(XDG_BIN_HOME:/bin=),$(HOME)/.local)
CARGO ?= cargo
NPM ?= npm
WEB_SKIP_INSTALL ?= 0
AUR_ROOTFS ?= archlinux
AUR_REV ?= HEAD
LINUX_TARGET ?= x86_64-unknown-linux-gnu
DEB_TARGET ?= $(LINUX_TARGET)
RPM_TARGET ?= $(LINUX_TARGET)
ARCHPKG_TARGET ?= $(LINUX_TARGET)
CHAN_TARGET ?=

# Linux chan-desktop build (AppImage/.deb) runs inside an sdme container so a
# macOS workstation can produce Linux bundles. DISTRO selects the rootfs +
# .sdme template; SDME is how sdme is reached (a lima VM on macOS, directly on
# a Linux host). See packaging/sdme/build-chan-desktop.sh.
DISTRO ?= ubuntu
SDME ?= limactl shell default sudo sdme

BIN := target/release/chan
WEB_BUILD_STAMP := web/.chan-build-stamp
LAUNCHER_BUILD_STAMP := web-launcher/.chan-build-stamp
REPO_ROOT := $(abspath .)

# Gateway release crate set. Single source for the pre-push gateway
# build (gateway-build) and the release.yml deb-packaging matrix, which
# both read it instead of repeating the names. The drive->workspace
# crate rename drifted release.yml off the real crate names and only
# surfaced at release-tag time, because nothing local built the gateway.
# Pointing every consumer here means a future rename breaks the local
# gate, not just the published release.
GATEWAY_RELEASE_CRATES := profile identity devserver-proxy admin

.PHONY: help
help: ## Show this help.
	@printf "chan build and release targets\n\n"
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_.-]+:.*##/ {printf "  %-28s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: chan
chan: web ## Build the release CLI binary.
	@if [ -n "$(CHAN_TARGET)" ]; then \
		$(CARGO) build --release --target "$(CHAN_TARGET)" -p chan; \
	else \
		$(CARGO) build --release -p chan; \
	fi

.PHONY: chan-desktop
chan-desktop: ## Build the desktop app through desktop/Makefile.
	$(MAKE) -C desktop build

.PHONY: desktop-dev
desktop-dev: ## Launch the desktop app in dev mode.
	$(MAKE) -C desktop dev

.PHONY: linux-chan-tarball
linux-chan-tarball: ## Build the Linux CLI tarball for LINUX_TARGET.
	$(MAKE) -C packaging/linux \
		CHAN_REPO="$(REPO_ROOT)" CARGO="$(CARGO)" NPM="$(NPM)" \
		LINUX_TARGET="$(LINUX_TARGET)" chan-tarball

.PHONY: linux-deb
linux-deb: ## Build a .deb for DEB_TARGET, defaulting to LINUX_TARGET.
	$(MAKE) -C packaging/linux \
		CHAN_REPO="$(REPO_ROOT)" CARGO="$(CARGO)" NPM="$(NPM)" \
		DEB_TARGET="$(DEB_TARGET)" deb

.PHONY: linux-rpm
linux-rpm: ## Build an .rpm for RPM_TARGET, defaulting to LINUX_TARGET.
	$(MAKE) -C packaging/linux \
		CHAN_REPO="$(REPO_ROOT)" CARGO="$(CARGO)" NPM="$(NPM)" \
		RPM_TARGET="$(RPM_TARGET)" rpm

.PHONY: linux-archpkg
linux-archpkg: ## Build an Arch package for ARCHPKG_TARGET.
	$(MAKE) -C packaging/linux \
		CHAN_REPO="$(REPO_ROOT)" CARGO="$(CARGO)" NPM="$(NPM)" \
		ARCHPKG_TARGET="$(ARCHPKG_TARGET)" archpkg

.PHONY: linux-packages
linux-packages: ## Build all Linux packages for the current target set.
	$(MAKE) -C packaging/linux \
		CHAN_REPO="$(REPO_ROOT)" CARGO="$(CARGO)" NPM="$(NPM)" \
		DEB_TARGET="$(DEB_TARGET)" RPM_TARGET="$(RPM_TARGET)" \
		ARCHPKG_TARGET="$(ARCHPKG_TARGET)" packages

.PHONY: aur-check
aur-check: ## Build and smoke both AUR packages in a disposable sdme Arch container.
	AUR_ROOTFS="$(AUR_ROOTFS)" REV="$(AUR_REV)" SDME="$(SDME)" \
		packaging/distros/arch/build-with-sdme.sh

.PHONY: linux-chan-desktop
linux-chan-desktop: ## Build the chan-desktop AppImage/.deb for DISTRO via sdme.
	$(MAKE) -C packaging/linux \
		CHAN_REPO="$(REPO_ROOT)" SDME="$(SDME)" DISTRO="$(DISTRO)" \
		chan-desktop

.PHONY: linux-gateway
linux-gateway: ## Build the gateway .deb packages via sdme (the gateway-linux-packages mirror).
	# The gateway is a separate nested workspace, so its sdme build infra
	# lives under packaging/gateway/scripts/dev/sdme/ (next to chan-psql.sdme) rather
	# than packaging/linux. SDME selects how sdme is reached (lima on macOS).
	CHAN_REPO="$(REPO_ROOT)" SDME="$(SDME)" \
		packaging/gateway/scripts/dev/sdme/build-gateway.sh

.PHONY: distros-tarball
distros-tarball: ## Build the vendored source tarball (COPR/PPA input) under target/distros.
	packaging/distros/mkdist --repo "$(REPO_ROOT)"

.PHONY: copr-srpm
copr-srpm: ## Build the chan + chan-desktop SRPMs locally (fedora container).
	packaging/distros/copr/build-srpm.sh $(PKG)

.PHONY: copr-build
copr-build: ## Build the SRPMs and submit them to COPR (needs copr-cli auth).
	packaging/distros/copr/build-srpm.sh $(PKG) --submit

.PHONY: ppa-source
ppa-source: ## Build signed per-series Launchpad source packages from the tarball.
	packaging/distros/debian/build-source.sh $(PKG)

.PHONY: ppa-upload
ppa-upload: ## dput the built source packages to the Launchpad PPA.
	packaging/distros/debian/upload.sh

.PHONY: macos-chan-app
macos-chan-app: ## Build and sign the macOS .app bundle.
	$(MAKE) -C desktop app-signed

.PHONY: macos-chan-dmg
macos-chan-dmg: ## Build the macOS .dmg bundle.
	$(MAKE) -C desktop dmg-layout-proof

.PHONY: macos-chan-dmg-notarised
macos-chan-dmg-notarised: ## Build, notarise, and staple the macOS .dmg.
	$(MAKE) -C desktop app-notarized

.PHONY: macos-chan-dmg-notarized
macos-chan-dmg-notarized: macos-chan-dmg-notarised

.PHONY: windows-chan-installer
windows-chan-installer: ## Build the Windows NSIS desktop installer.
	$(MAKE) -C desktop windows-installer

.PHONY: pre-push
pre-push: ## Run the local pre-push gate.
	$(CARGO) fmt --check
	RUSTFLAGS="-D warnings" $(CARGO) clippy --all-targets -- -D warnings
	RUSTFLAGS="-D warnings" $(CARGO) test --all-targets
	RUSTFLAGS="-D warnings" $(CARGO) build --no-default-features
	RUSTFLAGS="-D warnings" $(MAKE) gateway-build
	$(MAKE) web-check
	$(MAKE) web-marketing-check

.PHONY: ci-linux
ci-linux: pre-push ## Run the Linux CI validation target.

.PHONY: ci-macos
ci-macos: ## Run the focused macOS CI validation target.
	RUSTFLAGS="-D warnings" $(CARGO) clippy --all-targets -- -D warnings
	RUSTFLAGS="-D warnings" $(CARGO) test --all-targets

.PHONY: ci-release
ci-release: pre-push ## Run the local release validation target.

.PHONY: gateway-spa
gateway-spa: ## Build the gateway identity SPA bundle (rust-embed input).
	cd web && $(NPM) install && $(NPM) run build -w @chan/profile

.PHONY: gateway-build
gateway-build: gateway-spa ## Build the gateway release crates (GATEWAY_CARGO_FLAGS adds cross/release).
	# Depends on gateway-spa: identity embeds web/dist via rust-embed at
	# compile time, so the bundle must exist or the derive fails to build.
	cd gateway && $(CARGO) build $(GATEWAY_CARGO_FLAGS) \
		$(foreach crate,$(GATEWAY_RELEASE_CRATES),-p $(crate))

.PHONY: gateway-release-crates
gateway-release-crates: ## Print the gateway release crate names on one line.
	@echo $(GATEWAY_RELEASE_CRATES)

.PHONY: web-launcher
web-launcher: ## Build the embedded launcher bundle (web-launcher/dist).
	# chan-server bakes BOTH frontend bundles via rust-embed: web/dist
	# (WebAssets) and web-launcher/dist (LauncherAssets, the devserver/library
	# root SPA). web-launcher/dist is a gitignored build artifact, so every
	# path that builds web/dist before the cargo/rust-embed step must build
	# this too -- wired as a prerequisite of `web`/`web-check` so the single
	# `make web` funnel (root `chan`, desktop/Makefile, packaging/linux,
	# release.yml) builds both with no per-consumer edit.
	@if [ "$(WEB_SKIP_INSTALL)" != "1" ]; then cd web && $(NPM) install; fi
	cd web && $(NPM) run build -w @chan/launcher
	@date -u '+%Y-%m-%dT%H:%M:%SZ' > "$(LAUNCHER_BUILD_STAMP)"

.PHONY: web
web: web-launcher ## Build the embedded web bundle.
	@if [ "$(WEB_SKIP_INSTALL)" != "1" ]; then cd web && $(NPM) install; fi
	cd web && $(NPM) run build -w @chan/workspace-app
	@date -u '+%Y-%m-%dT%H:%M:%SZ' > "$(WEB_BUILD_STAMP)"

.PHONY: web-check
web-check: web-launcher ## Run frontend check, vitest, and production build.
	# vitest (npm test == `vitest run`) gates here so the pre-push / ci-linux
	# path covers the frontend unit tests. The Make gate previously ran only
	# svelte-check + build, leaving vitest ungated after CI was simplified to
	# the make ci-* targets. The `web-launcher` prerequisite builds the launcher
	# bundle so the pre-push / release cargo build embeds a real launcher.
	#
	# The web-launcher prerequisite only BUILDS the launcher (vite build), which
	# misses type errors + unit regressions, so gate its svelte-check + vitest
	# here too (it already ran `npm install`). Both SPAs are now fully gated.
	cd web && $(NPM) install \
		&& $(NPM) run check -w @chan/launcher && $(NPM) run test -w @chan/launcher \
		&& $(NPM) run check -w @chan/workspace-app && $(NPM) run test -w @chan/workspace-app \
		&& $(NPM) run build -w @chan/workspace-app
	@date -u '+%Y-%m-%dT%H:%M:%SZ' > "$(WEB_BUILD_STAMP)"

.PHONY: web-marketing-check
web-marketing-check: ## Run marketing site checks.
	cd web && $(NPM) install && $(NPM) run check -w @chan/marketing

.PHONY: models
models: ## Pre-fetch the optional embedded search model.
	$(CARGO) run --release -p fetch-models

.PHONY: build-release
build-release: models web ## Build chan with the embedded search model.
	$(CARGO) build --release --features embed-model -p chan

.PHONY: test
test: ## Run Rust tests.
	$(CARGO) test --workspace

.PHONY: lint
lint: ## Run Rust formatting and clippy checks.
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets -- -D warnings

.PHONY: hooks
hooks: ## Install the git pre-push hook.
	./scripts/install-hooks

.PHONY: install
install: chan ## Install chan under PREFIX/bin.
	install -d $(PREFIX)/bin
	install -m 755 $(BIN) $(PREFIX)/bin/chan
	@echo "installed to $(PREFIX)/bin/chan"
	@case ":$$PATH:" in *":$(PREFIX)/bin:"*) ;; \
		*) echo "note: $(PREFIX)/bin is not in PATH; add it to your shell rc";; \
	esac

.PHONY: uninstall
uninstall: ## Remove chan from PREFIX/bin.
	rm -f $(PREFIX)/bin/chan
	@echo "removed $(PREFIX)/bin/chan"

.PHONY: clean
clean: ## Remove local build outputs (root workspace, web, gateway, desktop).
	$(CARGO) clean
	rm -rf web/dist web/node_modules web/pkg
	rm -rf web-launcher/dist web-launcher/node_modules
	rm -f $(WEB_BUILD_STAMP) $(LAUNCHER_BUILD_STAMP)
	# gateway/ is its own cargo workspace: root `cargo clean` never
	# touches gateway/target. The gateway frontend now lives in the ./web
	# npm workspace; only its rust-embed SPA dist remains under gateway/.
	cd gateway && $(CARGO) clean
	rm -rf gateway/crates/identity/web/dist
	# Desktop owns its extras (downloaded sidecar binaries); same
	# delegation as the chan-desktop / desktop-dev build targets.
	$(MAKE) -C desktop clean

.PHONY: dev
dev: chan ## Run chan open against /tmp/chan-dev with no token.
	$(BIN) open /tmp/chan-dev --no-token

.PHONY: all build rpm
all: chan
build: chan
rpm: linux-rpm
