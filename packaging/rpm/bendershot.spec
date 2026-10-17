%{!?_userunitdir: %global _userunitdir /usr/lib/systemd/user}

Name:           bendershot
Version:        0.3.1
Release:        0%{?dist}
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
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/256x256/apps
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/64x64/apps
mkdir -p %{buildroot}%{_datadir}/pixmaps
mkdir -p %{buildroot}%{_userunitdir}
mkdir -p %{buildroot}%{_sysconfdir}/xdg/autostart

install -m 755 %{_sourcedir}/bendershot %{buildroot}%{_bindir}/bendershot
ln -sf bendershot %{buildroot}%{_bindir}/enhanced-shot
install -m 644 %{_sourcedir}/bendershot.desktop %{buildroot}%{_datadir}/applications/bendershot.desktop
install -m 644 %{_sourcedir}/bendershot-autostart.desktop %{buildroot}%{_sysconfdir}/xdg/autostart/bendershot-autostart.desktop
install -m 644 %{_sourcedir}/bendershot.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/bendershot.svg
install -m 644 %{_sourcedir}/bendershot.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/bendershot.png
install -m 644 %{_sourcedir}/bendershot_64.png %{buildroot}%{_datadir}/icons/hicolor/64x64/apps/bendershot.png
install -m 644 %{_sourcedir}/bendershot.png %{buildroot}%{_datadir}/pixmaps/bendershot.png
install -m 644 %{_sourcedir}/bendershot.service %{buildroot}%{_userunitdir}/bendershot.service

%post
if [ -x %{_bindir}/update-desktop-database ]; then
    %{_bindir}/update-desktop-database %{_datadir}/applications &>/dev/null || :
fi
if [ -x %{_bindir}/gtk-update-icon-cache ]; then
    %{_bindir}/gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor &>/dev/null || :
fi

# Enable systemd user service globally for desktop sessions
if [ -x %{_bindir}/systemctl ]; then
    %{_bindir}/systemctl --global daemon-reload &>/dev/null || :
    %{_bindir}/systemctl --global enable bendershot.service &>/dev/null || :
fi

# Detect target desktop user (either via sudo or active graphical session)
TARGET_USER="${SUDO_USER:-}"
if [ -z "$TARGET_USER" ] || [ "$TARGET_USER" = "root" ]; then
    TARGET_USER=$(loginctl list-users --no-legend 2>/dev/null | awk '$2 != "root" {print $2; exit}' || :)
fi

if [ -n "$TARGET_USER" ] && [ "$TARGET_USER" != "root" ]; then
    TARGET_UID=$(id -u "$TARGET_USER" 2>/dev/null || :)
    if [ -n "$TARGET_UID" ]; then
        systemctl --user -M "${TARGET_USER}@" daemon-reload &>/dev/null || :
        systemctl --user -M "${TARGET_USER}@" reset-failed bendershot.service &>/dev/null || :
        systemctl --user -M "${TARGET_USER}@" enable --now bendershot.service &>/dev/null || :
        su - "$TARGET_USER" -c "export XDG_RUNTIME_DIR=/run/user/${TARGET_UID}; export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/${TARGET_UID}/bus; busctl --user call org.freedesktop.impl.portal.PermissionStore /org/freedesktop/impl/portal/PermissionStore org.freedesktop.impl.portal.PermissionStore SetPermission sbssas screenshot true screenshot bendershot 1 yes 2>/dev/null || true; %{_bindir}/bendershot auto-setup" &>/dev/null || :
        systemctl --user -M "${TARGET_USER}@" restart org.gnome.SettingsDaemon.MediaKeys.target &>/dev/null || :
    fi
fi

%postun
if [ -x %{_bindir}/update-desktop-database ]; then
    %{_bindir}/update-desktop-database %{_datadir}/applications &>/dev/null || :
fi
if [ -x %{_bindir}/gtk-update-icon-cache ]; then
    %{_bindir}/gtk-update-icon-cache -f -t %{_datadir}/icons/hicolor &>/dev/null || :
fi
if [ $1 -eq 0 ]; then
    if [ -x %{_bindir}/systemctl ]; then
        %{_bindir}/systemctl --global disable bendershot.service &>/dev/null || :
    fi
    TARGET_USER="${SUDO_USER:-}"
    if [ -z "$TARGET_USER" ] || [ "$TARGET_USER" = "root" ]; then
        TARGET_USER=$(loginctl list-users --no-legend 2>/dev/null | awk '$2 != "root" {print $2; exit}' || :)
    fi
    if [ -n "$TARGET_USER" ] && [ "$TARGET_USER" != "root" ]; then
        systemctl --user -M "${TARGET_USER}@" stop bendershot.service &>/dev/null || :
        systemctl --user -M "${TARGET_USER}@" disable bendershot.service &>/dev/null || :
    fi
fi

%files
%{_bindir}/bendershot
%{_bindir}/enhanced-shot
%{_datadir}/applications/bendershot.desktop
%{_sysconfdir}/xdg/autostart/bendershot-autostart.desktop
%{_datadir}/icons/hicolor/scalable/apps/bendershot.svg
%{_datadir}/icons/hicolor/256x256/apps/bendershot.png
%{_datadir}/icons/hicolor/64x64/apps/bendershot.png
%{_datadir}/pixmaps/bendershot.png
%{_userunitdir}/bendershot.service

%changelog
* Sun Oct 11 2026 BenderShot Maintainers <contact@bendershot.org> - 0.1.0-1
- Rebranding to BenderShot with advanced vector tools and organic transitions
