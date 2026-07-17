# Spec for chan-desktop, the Tauri desktop shell, built offline from the
# same vendored source tarball as the chan CLI (packaging/distros/mkdist).
# The build tooling rewrites %%upstream_version from the workspace
# Cargo.toml before rpmbuild, so the committed value is a fallback.
#
# The desktop binary IS the chan/cs command line (argv0 dispatch), so this
# package ships /usr/bin/{chan,cs} symlinks and conflicts with the chan
# package instead of depending on it.

%global debug_package %{nil}
%global upstream_version 0.70.0

Name:           chan-desktop
Version:        %(echo %{upstream_version} | tr - '~')
Release:        1%{?dist}
Summary:        Desktop edition of the chan AI-native IDE
License:        Apache-2.0
URL:            https://chan.app
Source0:        chan-vendored-%{upstream_version}.tar.xz
ExclusiveArch:  x86_64 aarch64

BuildRequires:  rust >= 1.95
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  gcc-c++
BuildRequires:  pkgconf-pkg-config
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  gtk3-devel
BuildRequires:  libsoup3-devel
BuildRequires:  desktop-file-utils
BuildRequires:  systemd-rpm-macros

Requires:       systemd
# Both packages own %%{_bindir}/{chan,cs}; the desktop binary provides the
# full CLI in-process, so install one or the other.
Conflicts:      chan

%description
chan-desktop is the native desktop shell for chan, the AI-native IDE for
the modern engineer. It embeds the chan server and web app in a WebKitGTK
window and doubles as the chan/cs command line: invoked as chan or cs it
dispatches the CLI before any GUI init.

%prep
%autosetup -n chan-%{upstream_version}

%build
export CARGO_HOME="$PWD/.cargo-home"
export CHAN_PACKAGED=rpm
cargo build --release --frozen -p chan-desktop

%install
install -Dm755 target/release/chan-desktop %{buildroot}%{_bindir}/chan-desktop
ln -s chan-desktop %{buildroot}%{_bindir}/chan
ln -s chan-desktop %{buildroot}%{_bindir}/cs
install -Dm644 packaging/distros/shared/chan-desktop.desktop \
    %{buildroot}%{_datadir}/applications/chan-desktop.desktop
install -Dm644 desktop/src-tauri/icons/32x32.png \
    %{buildroot}%{_datadir}/icons/hicolor/32x32/apps/chan-desktop.png
install -Dm644 desktop/src-tauri/icons/64x64.png \
    %{buildroot}%{_datadir}/icons/hicolor/64x64/apps/chan-desktop.png
install -Dm644 desktop/src-tauri/icons/128x128.png \
    %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/chan-desktop.png
install -Dm644 desktop/src-tauri/icons/128x128@2x.png \
    %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/chan-desktop.png
install -Dm644 desktop/src-tauri/icons/icon.png \
    %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/chan-desktop.png
install -Dm644 packaging/distros/shared/chan-devserver.service \
    %{buildroot}%{_userunitdir}/chan-devserver.service

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/chan-desktop.desktop

%post
%systemd_user_post chan-devserver.service

%preun
%systemd_user_preun chan-devserver.service

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/chan-desktop
%{_bindir}/chan
%{_bindir}/cs
%{_datadir}/applications/chan-desktop.desktop
%{_datadir}/icons/hicolor/*/apps/chan-desktop.png
%{_userunitdir}/chan-devserver.service

%changelog
* Fri Jul 17 2026 Alexandre Fiori <fiorix@gmail.com> - 0.70.0-1
- Update to 0.70.0.
* Thu Jul 16 2026 Alexandre Fiori <fiorix@gmail.com> - 0.69.1-1
- Update to 0.69.1.
* Wed Jul 15 2026 Alexandre Fiori <fiorix@gmail.com> - 0.69.0-1
- Update to 0.69.0.
* Wed Jul 15 2026 Alexandre Fiori <fiorix@gmail.com> - 0.68.0-1
- Update to 0.68.0.
* Mon Jul 13 2026 Alexandre Fiori <fiorix@gmail.com> - 0.67.3-1
- Update to 0.67.3.
* Sun Jul 12 2026 Alexandre Fiori <fiorix@gmail.com> - 0.67.2-1
- Update to 0.67.2.
* Sun Jul 12 2026 Alexandre Fiori <fiorix@gmail.com> - 0.67.1-1
- Update to 0.67.1.
* Sat Jul 11 2026 Alexandre Fiori <fiorix@gmail.com> - 0.67.0-1
- Update to 0.67.0.
* Fri Jul 10 2026 Alexandre Fiori <fiorix@gmail.com> - 0.66.1-1
- Initial COPR packaging.
