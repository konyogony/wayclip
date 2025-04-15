<div align="center">
    <h1>Wayclip</h1>
    <img alt="Coding Time For Wayclip" src="https://wakapi.dev/api/badge/konyogony/interval:any/project:wayclip" />
    <img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg" />
</div>

# Prerequisites

- [Wayland](https://wayland.freedesktop.org/), works _so far only_ on Wayland. I may add more support for X11 in the future.
- [xdg-desktop-portal](https://wiki.archlinux.org/title/XDG_Desktop_Portal), you can use any xdg-desktop-portal, make sure it allows for screencast
- [Hyprland](https://hyprland.org/) **OPTIONAL**, but more recommended, since ive tested this only for it, and their [portal](https://archlinux.org/packages/?name=xdg-desktop-portal-hyprland) supports screencast, as well as [hyprctl](https://wiki.hyprland.org/Configuring/Using-hyprctl/) allows for keybinding.

# Roadmap

- [ ] Background proccess (daemon)
  - [ ] Record the Background
  - [ ] Place frames in a circular buffer
  - [ ] Listen for keyboard shortcuts
  - [ ] Dump the buffer to a file
  - [ ] Start a new buffer
  - [ ] Audio (gonna be hard)
  - [ ] Pls use less than 3GB RAM
  - [ ] Restore token? whatever that is
  - [x] Pull settings from `.config/wayclip/settings.json`

- [ ] GUI using tauri
  - [x] Settup tauri and vite environment
  - [x] Sidebar & Routing
  - [ ] Show all saved clips
  - [ ] Rename, delete, play, cut clips
  - [ ] Upload / Share clips
  - [ ] Notifications
  - [ ] Start / Stop, save recording
  - [x] Settings
    - [x] Clip name formatting
    - [x] Video format (or maybe enforce only mp4)
    - [x] Clip length (30s, 1m, 2m, 5m, 10m)
    - [x] Storage location
      - [ ] Maximum storage size
    - [x] Clip quality
      - [x] Resolution
      - [x] FPS
      - [x] Include audio?

- [ ] Website (NextJS)
    - [ ] Information
    - [ ] Docs (Doxium)
     
- [ ] Misc
    - [ ] Option to save the clips somewhere online on a cdn
    - [ ] Share in an embed
    - [ ] Discord server (better to use konyogony.dev)
