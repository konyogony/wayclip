<img alt="Coding Time For Wayclip" src="https://wakapi.dev/api/badge/konyogony/interval:any/project:wayclip">

# Prerequisites

- [Wayland](https://wayland.freedesktop.org/), works _only_ on Wayland
- [xdg-desktop-portal-hyprland](https://archlinux.org/packages/?name=xdg-desktop-portal-hyprland), you can use any xdg-desktop-portal, make sure it allows for screencast and input capture. You check them [here](https://wiki.archlinux.org/title/XDG_Desktop_Portal)

# Roadmap

1. dbus connection to xdg-desktop-portal
2. request to capture screen
3. pipe screenshare to ffmpeg as rawvideo
4. ffmpeg uses segment_format and segment_wrap to have a circular buffer of a set size, somehow do it in ram?
5. detect keybaord shortcut and save ffmpeg file permamently in set location
6. restart ffmpeg buffer

- [ ] Background proccess (daemon)

  - [x] Pull settings from `.config/wayclip/settings.conf`

- [ ] GUI using tauri
  - [x] Settup tauri and vite environment
  - [x] Sidebar & Routing
  - [ ] Show all saved clips
  - [ ] Rename, delete, play, cut clips
  - [ ] Upload / Share clips
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
