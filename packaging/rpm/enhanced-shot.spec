Name:           enhanced-shot
Version:        0.1.0
Release:        1%{?dist}
Summary:        Captura e anotação profissional de tela para Linux Wayland
License:        MIT OR Apache-2.0
URL:            https://github.com/gfidelis/enhanced-shot
BuildArch:      x86_64

AutoReqProv:    no

%description
Ferramenta nativa em Rust para captura e anotação imediata de tela
no Linux Wayland, inspirada no Lightshot e Spectacle, estilizada
em GNOME Libadwaita com suporte a modo escuro/claro, camadas vetoriais
não-destrutivas, ofuscação (desfoque, mosaico, smudge), seletor de cores
e janela flutuante fixável.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
mkdir -p %{buildroot}%{_userunitdir}

install -m 755 %{_sourcedir}/enhanced-shot %{buildroot}%{_bindir}/enhanced-shot
install -m 644 %{_sourcedir}/enhanced-shot.desktop %{buildroot}%{_datadir}/applications/enhanced-shot.desktop
install -m 644 %{_sourcedir}/enhanced-shot.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/enhanced-shot.svg
install -m 644 %{_sourcedir}/enhanced-shot.service %{buildroot}%{_userunitdir}/enhanced-shot.service

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
%{_bindir}/enhanced-shot
%{_datadir}/applications/enhanced-shot.desktop
%{_datadir}/icons/hicolor/scalable/apps/enhanced-shot.svg
%{_userunitdir}/enhanced-shot.service

%changelog
* Sat Oct 10 2026 Enhanced Shot Maintainers <contact@enhancedshot.org> - 0.1.0-1
- Initial release for Linux Wayland
