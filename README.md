# Weather · With 30 Years of "On This Day" History

**[中文](README.zh.md) | English**

A purely front-end static weather website (bilingual Chinese/English) that can be wrapped into a desktop app, a mobile app, or a WeChat Mini Program. Look up current weather and a 7-day forecast for any city, plus **historical data for today's date over the past 30 years** for the selected city.

Microsoft's MSN Weather used to be free of annoying news feeds and terrible ads — a clean, genuinely useful weather tool. It even had some unique features, like showing the number of precipitation days and the highest or lowest temperatures on this date over the past thirty years. Sadly, the useful features were retired while the useless ads took over the screen. So I wanted a weather assistant of my own — one that makes up for the shortcomings of common apps like weather.com.cn and Moji Weather, with the cards I like.

## Features

- **Chinese/English toggle**: one click in the top bar, remembered locally; city names, weather descriptions, level wording, and the air-quality standard all follow the language.
- **Auto location**: on page load, the site requests the user's location and shows local weather (with browser permission); on denial or failure it falls back to Beijing with a toast. The 📍 button in the top bar re-locates at any time. The last viewed location is remembered and loaded directly on the next visit. City names are reverse-geocoded via BigDataCloud's free client-side API.
- **City search**: bilingual autocomplete (Open-Meteo Geocoding), defaults to Beijing.
- **Common weather cards**:
  - Current weather hero card (temperature, weather icon, feels-like, description)
  - Humidity: 270° gauge arc (needle + color-coded segments)
  - Wind: Beaufort level + MSN-style tick compass (speed & direction merged card)
  - Pressure / UV / Air Quality: segmented indicator bars
  - Air Quality: Chinese uses China's national standard (HJ 633-2012, with primary pollutant); English uses US AQI
  - Moon phase: 8 phases + illumination percentage + SVG moon graphic (computed locally)
  - Sunrise / Sunset: merged two-column card with a sun-trajectory arc showing the sun's position today
  - 7-day forecast (icon, high/low, precipitation chance; card / list modes)
- **"On this day" cards for the past 30 years** (uses the selected city's local date; statistics for that date over the past 30 years):
  - 🌧️ Precipitation days: how many of the past 30 years had precipitation on this date; switches to "snow days" in winter, counting snowfall instead
  - 🌧️ Max precipitation: the largest precipitation on this date in 30 years, with year and date
  - 🔥 Highest temperature: the highest temperature on this date in 30 years, with year and date
  - ❄️ Lowest temperature: the lowest temperature on this date in 30 years, with year and date

## Performance & Caching

- The 30-year history is roughly 10,000 daily records; a single full fetch takes about 35 seconds. This site splits the window into **6 five-year chunks fetched concurrently**, cutting first load to ~6s; the raw data is then stored in `localStorage` and cached for **30 days**, making subsequent visits instant.
- Current weather is cached for **10 minutes** and air quality for **30 minutes** to avoid frequent requests.
- Weather and history are **fetched in parallel**; total time is the slower of the two (~6s).
- When "today" rolls over to a new day, the frontend recomputes from cached data — no refetch needed.

## Running Locally

No build step required — either works:

```bash
# Option 1: Python built-in static server
python3 -m http.server 8123
# Open http://localhost:8123 in a browser

# Option 2: Node
npx serve .
```

You can also double-click `index.html` to open it via `file://` (the APIs allow CORS, so it works fine). Note: browser Geolocation requires a secure context (HTTPS or localhost); most browsers disable it on `file://`, in which case the site falls back to Beijing.

## Desktop App

### Linux (Debian / Ubuntu / Kali):

```bash
# System deps + Rust (rustc >= 1.77.5; distro packages or rustup both work)
sudo apt install rustc cargo libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev

npm install
npx tauri dev      # run in development
npx tauri build    # produces .deb / AppImage under src-tauri/target/release/bundle/
```

### Windows

Install the prerequisites (one-time):

- **Rust**: run the [rustup-init.exe](https://rustup.rs) installer with default options (toolchain `stable-x86_64-pc-windows-msvc`)
- **Visual Studio 2022 Build Tools**: select the "Desktop development with C++" workload (MSVC v143 + Windows 11 SDK included), ~3-4 GB
- **Node.js** ≥ 18 (only used for the Tauri CLI)
- WebView2 ships with Windows 10/11 — nothing extra to install

Build:

```bash
npm install
npx tauri build
```

**Speeding up downloads from mainland China** (when direct access is slow or blocked):

```bash
# rustup components via the USTC mirror (reopen the terminal after setx, then rustup default stable-msvc)
setx RUSTUP_DIST_SERVER https://mirrors.ustc.edu.cn/rust-static
setx RUSTUP_UPDATE_ROOT https://mirrors.ustc.edu.cn/rust-static/rustup

# crates.io dependencies via the sparse mirror: write to %USERPROFILE%\.cargo\config.toml
# [source.crates-io]
# replace-with = 'ustc'
# [source.ustc]
# registry = 'sparse+https://mirrors.ustc.edu.cn/crates.io-index/'

# The first build also downloads the WiX / NSIS packers from GitHub;
# if that fails, proxy the build process:
set HTTPS_PROXY=http://127.0.0.1:10808
npx tauri build
```

Notes:

- On Linux, WebKitGTK geolocation relies on the geoclue service and often fails — the app keeps the web fallback to Beijing
- GNOME hides tray icons by default; install the AppIndicator/KStatusNotifier extension. KDE / XFCE support it natively
- The tray hover card works on all three platforms (Linux detects hover via cursor polling, so a slight delay is normal)
- Windows 11 tucks new apps into the hidden tray overflow by default; enable them under Taskbar settings → Other system tray icons, or drag the icon onto the taskbar
- The Windows taskbar weather strip is an always-on-top mini window and stays visible over fullscreen apps (videos, games); mention it if you'd rather have it removed
- Weather icons are emoji; install `fonts-noto-color-emoji` on Linux

## Data Sources

All from [Open-Meteo](https://open-meteo.com/) (free, open source, no API key, CORS enabled):

| Purpose | Endpoint |
| --- | --- |
| City search | `geocoding-api.open-meteo.com` |
| Current weather + 7-day forecast | `api.open-meteo.com/v1/forecast` |
| 30-year daily history | `archive-api.open-meteo.com/v1/archive` |
| Air quality (PM2.5/PM10/O₃/NO₂/SO₂/CO concentrations + US AQI) | `air-quality-api.open-meteo.com/v1/air-quality` |

## Notes

- Open-Meteo's free tier allows 10,000 non-commercial calls per day — plenty for personal use.
- Historical archive data may lag by a few days; recent days update as the archive is completed.
- Wind speed is measured at 10m, temperature at 2m, snowfall in cm, per WMO standards.
