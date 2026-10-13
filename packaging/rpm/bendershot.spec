%{!?_userunitdir: %global _userunitdir /usr/lib/systemd/user}

Name:           bendershot
Version:        0.1.3
Release:        1%{?dist}
Summary:        Captura, gravação e anotação profissional de tela para Linux Wayland
License:        MIT OR Apache-2.0
URL:            https://github.com/gfidelis/bendershot
BuildArch:      x86_64

AutoReqProv:    no
Provides:       enhanced-shot = %{version}-%{release}

%description
Ferramenta nativa em Rust para captura, gravação e anotação imediata de tela
no Linux Wayland, estilizada em GNOME Libadwaita com suporte a modo escuro/claro,
camadas vetoriais não-destrutivas, ferramentas avançadas de geometria (Bézier, Polígonos),
suavização de traço Chaikin, transições orgânicas write-on e fade-out, ofuscação
(desfoque, mosaico, smudge), seletor de cores e janela flutuante fixável.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
mkdir -p %{buildroot}%{_userunitdir}

install -m 755 %{_sourcedir}/bendershot %{buildroot}%{_bindir}/bendershot
ln -sf bendershot %{buildroot}%{_bindir}/enhanced-shot
install -m 644 %{_sourcedir}/bendershot.desktop %{buildroot}%{_datadir}/applications/bendershot.desktop
install -m 644 %{_sourcedir}/bendershot.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/bendershot.svg
install -m 644 %{_sourcedir}/bendershot.service %{buildroot}%{_userunitdir}/bendershot.service

%post
if [ -x %{_bindir}/update-desktop-database ]; then
    %{_bindir}/update-desktop-database %{_datadir}/applications &>/dev/null || :
fi
if [ -x %{_bindir}/gtk-update-icon-cache ]; then
    %{_bindir}/gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor &>/dev/null || :
fi

%postun
if [ -x %{_bindir}/update-desktop-database ]; then
    %{_bindir}/update-desktop-database %{_datadir}/applications &>/dev/null || :
fi
if [ -x %{_bindir}/gtk-update-icon-cache ]; then
    %{_bindir}/gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor &>/dev/null || :
fi

%files
%{_bindir}/bendershot
%{_bindir}/enhanced-shot
%{_datadir}/applications/bendershot.desktop
%{_datadir}/icons/hicolor/scalable/apps/bendershot.svg
%{_userunitdir}/bendershot.service

%changelog
* Sun Oct 11 2026 BenderShot Maintainers <contact@bendershot.org> - 0.1.0-1
- Rebranding to BenderShot with advanced vector tools and organic transitions
