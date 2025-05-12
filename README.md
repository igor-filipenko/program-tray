# program-tray
Wrap any program in tray for background work. Gtk native application.

Compile system dependencies:
```bash
sudo apt install libgtk-3-dev libxdo-dev libayatana-appindicator3-dev
```

Build:
```bash
cargo build --release
```

Usage:
```bash
program-tray some-program.toml
```

Example of TOML:
```toml
id = "some-program"
command = "some-program --user $user"
input = "$password"

[args]
user = "user"
password = "password"

[env]
var1 = "val1"

[ui]
title = "some program"

[ui.icons]
on = "/some/path/to/file"
off = "/some/path/to/file"
```

## How it can be use

Using file layout:
```
/opt/some-program/
│
├── program-tray          # This application
├── some-program.desktop  # Desktop entry file
├── some-program.png      # Application icon
└── some-program.toml     # Configuration file
```
Example of desktop entry:
```
[Desktop Entry]
Version=1.0
Type=Application
Name=Some Program
Icon=/opt/some-program/some-program.png
Exec=/opt/some-program/program-tray /opt/some-program/some-program.toml
Comment=Some Comment
Categories=GNOME;GTK;Utility;
Terminal=false
StartupNotify=true
```
Then install:
```bash
desktop-file-install --rebuild-mime-info-cache --dir=$HOME/.local/share/applications /opt/some-program/some-program.desktop
```