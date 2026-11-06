Name:           rustfetch
Version:        0.1.5
Release:        1%{?dist}
Summary:        A blazing-fast, memory-safe system fetch tool written in Rust

License:        MIT
URL:            https://github.com/MehmetCanWT/rustfetch
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.88.0
BuildRequires:  gcc

%description
RustFetch is a high-performance, modular system information tool written in pure
Rust. Designed as a memory-safe and zero-fork alternative to Neofetch and Fastfetch,
RustFetch gathers system telemetry directly from kernel interfaces (/proc and /sys)
without spawning shell sub-processes.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked -p rustfetch

%check
cargo test --release --workspace --locked

%install
# Binary
install -Dpm 0755 target/release/rustfetch %{buildroot}%{_bindir}/rustfetch
ln -sf rustfetch %{buildroot}%{_bindir}/rfetch

# Man page
install -Dpm 0644 docs/rustfetch.1 %{buildroot}%{_mandir}/man1/rustfetch.1
ln -sf rustfetch.1 %{buildroot}%{_mandir}/man1/rfetch.1

# Shell completions
install -d %{buildroot}%{_datadir}/bash-completion/completions
target/release/rustfetch --completions bash > %{buildroot}%{_datadir}/bash-completion/completions/rustfetch
ln -sf rustfetch %{buildroot}%{_datadir}/bash-completion/completions/rfetch

install -d %{buildroot}%{_datadir}/zsh/site-functions
target/release/rustfetch --completions zsh > %{buildroot}%{_datadir}/zsh/site-functions/_rustfetch
ln -sf _rustfetch %{buildroot}%{_datadir}/zsh/site-functions/_rfetch

install -d %{buildroot}%{_datadir}/fish/vendor_completions.d
target/release/rustfetch --completions fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/rustfetch.fish
ln -sf rustfetch.fish %{buildroot}%{_datadir}/fish/vendor_completions.d/rfetch.fish

%files
%license LICENSE
%doc README.md
%{_bindir}/rustfetch
%{_bindir}/rfetch
%{_mandir}/man1/rustfetch.1*
%{_mandir}/man1/rfetch.1*
%{_datadir}/bash-completion/completions/rustfetch
%{_datadir}/bash-completion/completions/rfetch
%{_datadir}/zsh/site-functions/_rustfetch
%{_datadir}/zsh/site-functions/_rfetch
%{_datadir}/fish/vendor_completions.d/rustfetch.fish
%{_datadir}/fish/vendor_completions.d/rfetch.fish

%changelog
* Fri Nov 06 2026 Mehmet Can <https://github.com/MehmetCanWT> - 0.1.5-1
- Sync with upstream v0.1.5 "Ragdoll"
- Zero-alloc ANSI parser, synchronized output, hardened shell hooks & tools

* Fri Nov 06 2026 Mehmet Can <https://github.com/MehmetCanWT> - 0.1.4-1
- Sync with upstream v0.1.4 "Siamese"
- Declare MSRV rust >= 1.88.0 (requires image 0.25.x)
- Run the full workspace test suite (--release --workspace --locked) in %check

* Wed Sep 30 2026 Mehmet Can <https://github.com/MehmetCanWT> - 0.1.0-1
- Initial release of RustFetch
