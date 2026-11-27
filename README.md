<div align="center">

# GlassWeather

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/secnotes/glassweather?label=Release)](https://github.com/secnotes/glassweather/releases)
[![Platform: Windows / Linux](https://img.shields.io/badge/Platform-Windows%20%2F%20Linux-lightgrey)](https://github.com/secnotes/glassweather/releases)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-orange)](https://v2.tauri.app/)
[![Data: Open-Meteo](https://img.shields.io/badge/Data-Open--Meteo-brightgreen)](https://open-meteo.com/)
[![Cities: DataV Adcode](https://img.shields.io/badge/Cities-DataV%20Adcode-9cf)](https://github.com/DataV)

**English | [中文](README.zh.md)**

A purely front-end static weather website (bilingual Chinese/English) that can be wrapped into a desktop app, a mobile app, or a WeChat Mini Program. Look up current weather and a 7-day forecast for any city, plus **historical data for today's date over the past 30 years** for the selected city.

</div>

> Microsoft's MSN Weather used to be free of annoying news feeds and terrible ads — a clean, genuinely useful weather tool. It even had some unique features, like showing the number of precipitation days and the highest or lowest temperatures on this date over the past thirty years. Sadly, the useful features were retired while the useless ads took over the screen. So I wanted a weather assistant of my own — one that makes up for the shortcomings of common apps like weather.com.cn and Moji Weather, with the cards I like.


## Build

### Linux (Debian / Ubuntu / Kali)

```bash
# System deps + Rust (rustc >= 1.77.5; distro packages or rustup both work)
sudo apt install rustc cargo libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev

npm install
npx tauri dev      # run in development
npx tauri build    # produces .deb / AppImage under src-tauri/target/release/bundle/

# Weather icons are emoji; install fonts-noto-color-emoji on Linux
```

### Windows

Install the prerequisites (one-time)

- **Rust**: run the [rustup-init.exe](https://rustup.rs) installer with default options (toolchain `stable-x86_64-pc-windows-msvc`)
- **Visual Studio 2022 Build Tools**: select the "Desktop development with C++" workload (MSVC v143 + Windows 11 SDK included), ~3-4 GB
- **Node.js** ≥ 18 (only used for the Tauri CLI)
- WebView2 ships with Windows 10/11 — nothing extra to install

Build

```bash
npm install
npx tauri build
```

<details>
<summary><b>Speeding up downloads from mainland China</b> (when direct access is slow or blocked)</summary>

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

</details>

