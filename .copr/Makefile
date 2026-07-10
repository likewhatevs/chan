# COPR SRPM entry point. COPR's "make srpm" source method invokes:
#
#   make -f <clone>/.copr/Makefile srpm outdir=<dir> spec=<path>
#
# as root in a network-enabled mock chroot, with the repo cloned fresh.
# The offline part of the build (rpmbuild from the SRPM) happens later in
# the per-chroot builders; this stage may download (npm install, cargo
# vendor). The COPR project defines two packages, chan and chan-desktop,
# differing only in the spec path they pass.

TOP := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))..)
outdir ?= $(TOP)/target/distros/srpm
spec ?= $(TOP)/packaging/distros/fedora/chan.spec

.PHONY: srpm
srpm:
	dnf -y install git-core make tar xz rust cargo nodejs npm rpm-build
	# Optional: prunes non-Linux crate payloads from vendor/ (~35 MB);
	# mkdist falls back to plain `cargo vendor` when absent.
	-dnf -y install cargo-vendor-filterer
	$(TOP)/packaging/distros/copr/make-srpm.sh \
		--repo "$(TOP)" --spec "$(spec)" --outdir "$(outdir)"
