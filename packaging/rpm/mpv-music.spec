%global debug_package %{nil}

Name:           mpv-music
Version:        0.29.1
Release:        2%{?dist}
Summary:        A TUI-based music player wrapper for MPV

License:        MIT
URL:            https://github.com/FurqanHun/mpv-music
Source0:        %{url}/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.93.0
BuildRequires:  gcc

Recommends:     mpv
Suggests:       yt-dlp

%description
mpv-music is a blazing-fast, terminal-native music player and library browser
written in Rust.

It indexes your music collection into a lightning-fast library, providing fuzzy
searching (via skim), metadata-rich previews, and deep integration with mpv for
high-quality playback.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release

%install
install -Dm0755 target/release/mpv-music %{buildroot}%{_bindir}/mpv-music

%files
%license LICENSE
%doc README.md CHANGELOG.md
%{_bindir}/mpv-music

%changelog
* Tue Oct 27 2026 FurqanHun <furqanhun@proton.me> - 0.29.1-2
- Demote yt-dlp from Recommends to Suggests so DNF does not install it by default

* Tue Oct 27 2026 FurqanHun <furqanhun@proton.me> - 0.29.1-1
- Release v0.29.1
- Upstream changelog: https://github.com/FurqanHun/mpv-music/releases/tag/v0.29.1
