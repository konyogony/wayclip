# Roadmap

- [ ] Background proccess (daemon)

  - [ ] Capture screen using pipewire and ffmpeg?
  - [ ] Listen for keyboard events using `evdev` or some other tool
  - [ ] Save last few minutes
  - [ ] Pull settings from `.config/wayclip/settings.conf`

- [ ] GUI using tauri
  - [x] Settup tauri and vite environment
  - [x] Sidebar & Routing
  - [ ] Show all saved clips
  - [ ] Rename, delete, play, cut clips
  - [ ] Upload / Share clips
  - [ ] Settings
    - [ ] Clip name formatting
      - [ ] Day
      - [ ] Month
      - [ ] Year
      - [ ] Hour
      - [ ] Minute
      - [ ] Second
      - [ ] Hash
      - [ ] Custom text
    - [ ] Video format (or maybe enforce only mp4)
    - [ ] Clip length (30s, 1m, 2m, 5m, 10m)
    - [ ] Storage location
      - [ ] Maximum storage size
    - [ ] Clip quality
      - [ ] Resolution
      - [ ] Bitrate
      - [ ] FPS
      - [ ] Include audio?
