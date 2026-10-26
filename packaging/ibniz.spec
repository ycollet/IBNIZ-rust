%global debug_package %{nil}

Name: ibniz
Version: %{package_version}
Release: %{package_release}%{?dist}
Summary: Live audiovisual programming with the IBNIZ virtual machine
License: Zlib
BuildArch: x86_64
Requires: libX11, libXcursor, libXi, libXrandr, libXinerama
Requires: mesa-libGL, mesa-libEGL
Requires: libwayland-client, libwayland-cursor, libwayland-egl
Requires: libxkbcommon, libxkbcommon-x11

%description
An altered Rust implementation of IBNIZ with a live editor, audio/video
playback, recording, bundled examples, a VM debugger and NSM session support.

%install
mkdir -p "%{buildroot}"
cp -a "%{stagedir}/." "%{buildroot}/"

%files
/usr/bin/ibniz
/usr/share/applications/ibniz.desktop
/usr/share/icons/hicolor/scalable/apps/ibniz.svg
/usr/share/icons/hicolor/256x256/apps/ibniz.png
%license /usr/share/doc/ibniz/LICENSE
%doc /usr/share/doc/ibniz/README.md
