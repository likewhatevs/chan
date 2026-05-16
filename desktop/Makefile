.PHONY: run build check fmt fmt-check lint test clean chan-bin \
        app-signed app-notarized sign-prereqs notarize-prereqs

# Monorepo root. Override via CHAN_REPO=/path on CI.
CHAN_REPO ?= ..
TARGET_TRIPLE := $(shell rustc -vV | sed -n 's/host: //p')
CHAN_BIN := src-tauri/binaries/chan-$(TARGET_TRIPLE)

# Stage chan as the bundled sidecar. Rebuilds every invocation so
# in-progress chan edits flow into `make run` without a separate
# step; cargo's own incremental build keeps the cost small when
# nothing changed.
chan-bin:
	cd $(CHAN_REPO)/web && npm run build
	cd $(CHAN_REPO) && cargo build --release --bin chan
	mkdir -p src-tauri/binaries
	cp $(CHAN_REPO)/target/release/chan $(CHAN_BIN)

run: chan-bin
	cd src-tauri && cargo tauri dev

build: chan-bin
	cd src-tauri && cargo tauri build

check:
	cd src-tauri && cargo check

fmt:
	cd src-tauri && cargo fmt --all

fmt-check:
	cd src-tauri && cargo fmt --all -- --check

lint:
	cd src-tauri && cargo clippy --all-targets -- -D warnings

test:
	cd src-tauri && cargo test --all-targets

clean:
	cd src-tauri && cargo clean
	rm -rf src-tauri/binaries

# ---------------------------------------------------------------------------
# Code signing + notarization (macOS, distribution to others).
# ---------------------------------------------------------------------------
#
# Signing is environment-driven; nothing personal lives in this repo.
#
#   APPLE_SIGNING_IDENTITY  full identity string from
#                           `security find-identity -v -p codesigning`,
#                           e.g. "Developer ID Application: Name (TEAMID)".
#                           Auto-detected from the keychain when a
#                           single "Developer ID Application: ..." cert
#                           is present; pin explicitly when there is
#                           more than one.
#   APPLE_TEAM_ID           the 10-char Team ID at the end of the
#                           identity string. Auto-derived from
#                           APPLE_SIGNING_IDENTITY; override only if
#                           the cert name does not follow the
#                           "... (TEAMID)" convention.
#   APPLE_ID                the Apple Developer account email
#   APPLE_PASSWORD          app-specific password generated at
#                           appleid.apple.com -> Sign-In and Security
#                           -> App-Specific Passwords. NOT the iCloud
#                           password. Treat as a credential.
#
# Recommended flow: stash APPLE_ID + APPLE_PASSWORD in your shell rc
# (~/.bashrc, ~/.zshrc, ...) or read APPLE_PASSWORD from the macOS
# keychain, e.g.
#
#   security add-generic-password -s chan-notary -a "$$APPLE_ID" -w
#   export APPLE_PASSWORD=$$(security find-generic-password \
#       -s chan-notary -a "$$APPLE_ID" -w)
#
# Two paths:
#
#   app-signed       sign the .app with Developer ID. Gatekeeper
#                    accepts on machines where the user has
#                    right-clicked -> Open at least once. Faster
#                    than notarized; useful for local install or
#                    dev distribution.
#   app-notarized    sign + send to Apple's notary service + staple
#                    the ticket + bundle a .dmg. The .dmg opens
#                    cleanly on any Mac with no Gatekeeper warnings.
#                    Roundtrip to Apple is 2-15 min per build.
#
# Both targets fail fast with a clear message if the required env
# vars are missing, so a half-built unsigned .app never gets handed
# off by accident.

# Auto-derive APPLE_SIGNING_IDENTITY from the keychain when not
# pinned. Two `security` calls (instead of `$(words ...)`) because
# the identity string contains spaces, which would break naive
# word-counting.
ifeq ($(APPLE_SIGNING_IDENTITY),)
DETECTED_SIGN_COUNT := $(shell security find-identity -v -p codesigning 2>/dev/null \
    | grep -c '"Developer ID Application:')
ifeq ($(DETECTED_SIGN_COUNT),1)
APPLE_SIGNING_IDENTITY := $(shell security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Developer ID Application:[^"]*\)".*/\1/p')
endif
endif
export APPLE_SIGNING_IDENTITY

# Auto-derive APPLE_TEAM_ID from the trailing "(TEAMID)" in the
# signing identity. Skipped when the identity is unset (the
# sign-prereqs target reports that case).
ifeq ($(APPLE_TEAM_ID),)
ifneq ($(APPLE_SIGNING_IDENTITY),)
APPLE_TEAM_ID := $(shell echo "$(APPLE_SIGNING_IDENTITY)" \
    | sed -n 's/.*(\([A-Z0-9]\{10\}\))$$/\1/p')
endif
endif
export APPLE_TEAM_ID

sign-prereqs:
	@if [ -z "$$APPLE_SIGNING_IDENTITY" ]; then \
	    echo "error: APPLE_SIGNING_IDENTITY not set"; \
	    echo "find one with: security find-identity -v -p codesigning"; \
	    echo "example: export APPLE_SIGNING_IDENTITY='Developer ID Application: Your Name (TEAMID)'"; \
	    exit 1; \
	fi
	@security find-identity -v -p codesigning | \
	    grep -F "$$APPLE_SIGNING_IDENTITY" >/dev/null || { \
	    echo "error: identity '$$APPLE_SIGNING_IDENTITY' not in keychain"; \
	    echo "available identities:"; \
	    security find-identity -v -p codesigning; \
	    exit 1; \
	}

notarize-prereqs:
	@if [ -z "$$APPLE_ID" ] || [ -z "$$APPLE_PASSWORD" ] || [ -z "$$APPLE_TEAM_ID" ]; then \
	    echo "error: notarization needs APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID"; \
	    echo "APPLE_ID       = developer account email"; \
	    echo "APPLE_PASSWORD = app-specific password (appleid.apple.com)"; \
	    echo "APPLE_TEAM_ID  = 10-char team id (auto-derived from APPLE_SIGNING_IDENTITY"; \
	    echo "                 when the cert name ends in '(TEAMID)')"; \
	    exit 1; \
	fi

# Sign the .app with the configured Developer ID identity. No
# notarization, no .dmg. Tauri picks APPLE_SIGNING_IDENTITY out of
# the env when calling `codesign`.
app-signed: sign-prereqs chan-bin
	cd src-tauri && cargo tauri build --bundles app
	@echo ""
	@echo "signed .app: src-tauri/target/release/bundle/macos/Chan.app"
	@echo "verify: codesign -dv --verbose=2 src-tauri/target/release/bundle/macos/Chan.app"

# Sign + notarize + staple + bundle a .dmg. Tauri's CLI handles the
# notary roundtrip automatically when APPLE_ID + APPLE_PASSWORD +
# APPLE_TEAM_ID are present in the env.
app-notarized: sign-prereqs notarize-prereqs chan-bin
	cd src-tauri && cargo tauri build --bundles app,dmg
	@echo ""
	@echo "notarized artifacts:"
	@ls -1 src-tauri/target/release/bundle/dmg/*.dmg 2>/dev/null || true
	@echo "verify: spctl -a -t open --context context:primary-signature -v \\"
	@echo "          src-tauri/target/release/bundle/macos/Chan.app"
