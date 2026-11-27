<div align="center">

# GlassWeather

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/secnotes/glassweather?label=Release)](https://github.com/secnotes/glassweather/releases)
[![Platform: Windows / Linux](https://img.shields.io/badge/Platform-Windows%20%2F%20Linux-lightgrey)](https://github.com/secnotes/glassweather/releases)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-orange)](https://v2.tauri.app/)
[![Data: Open-Meteo](https://img.shields.io/badge/Data-Open--Meteo-brightgreen)](https://open-meteo.com/)
[![Cities: DataV Adcode](https://img.shields.io/badge/Cities-DataV%20Adcode-9cf)](https://github.com/DataV)

**[English](README.md) | [中文](README.zh.md)**

一个纯前端静态天气网站（中英双语），可以开发成桌面应用，也可以开发成 App 或者微信小程序。查询任意城市的实时天气与未来 7 天预报，并展示**所选城市今天这个日期在过去 30 年的历史数据**。

</div>

> 曾经微软的 MSN 天气，没有烦人的新闻，糟糕的广告，而是有一个简洁、实用的天气软件。并且拥有一些独特的功能，例如可以查看过去三十年，这一天的降水天数、最高或最低温度。可惜后来，有用的功能下线了，没用的广告霸屏了。因此，我想拥有一个属于自己的天气助手，弥补常见的“中国天气网”、“墨迹天气”等 App 的不足，添加自己喜欢的卡片。

## 构建

### Linux（Debian / Ubuntu / Kali）

```bash
# 系统依赖 + Rust（rustc ≥ 1.77.5，apt 版或 rustup 均可）
sudo apt install rustc cargo libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev

npm install
npx tauri dev      # 开发运行
npx tauri build    # 产出 .deb / AppImage 于 src-tauri/target/release/bundle/

# 天气图标使用 emoji，Linux 需安装 `fonts-noto-color-emoji`
```


### Windows

安装依赖
   - **Rust**：运行 [rustup-init.exe](https://rustup.rs)，默认选项安装（工具链 `stable-x86_64-pc-windows-msvc`）
   - **Visual Studio 2022 生成工具**：勾选「使用 C++ 的桌面开发」工作负载（需含 MSVC v143 与 Windows 11 SDK），约 3-4 GB
   - **Node.js** ≥ 18（仅用于 Tauri CLI）
   - WebView2 为 Win10/11 系统自带，无需单独安装

构建
   ```bash
   npm install
   npx tauri build
   ```


<details>
<summary><b>国内网络提速</b>（直连慢/被拒时）</summary>

```bash
# rustup 组件下载走中科大镜像（设完重开终端再执行 rustup default stable-msvc）
setx RUSTUP_DIST_SERVER https://mirrors.ustc.edu.cn/rust-static
setx RUSTUP_UPDATE_ROOT https://mirrors.ustc.edu.cn/rust-static/rustup

# crates.io 依赖走 sparse 镜像：写入 %USERPROFILE%\.cargo\config.toml
# [source.crates-io]
# replace-with = 'ustc'
# [source.ustc]
# registry = 'sparse+https://mirrors.ustc.edu.cn/crates.io-index/'

# 首次打包时 Tauri 还要从 GitHub 下载 WiX / NSIS 工具包，
# 不通则给构建进程挂代理：
set HTTPS_PROXY=http://127.0.0.1:10808
npx tauri build
```

</details>
