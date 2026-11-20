# 天气查询 · 含过去 30 年今日天气

**中文 | [English](README.md)**

一个纯前端静态天气网站（中英双语），可部署在 GitHub Pages，也可以开发成 App 或者微信小程序。查询任意城市的实时天气与未来 7 天预报，并展示**所选城市今天这个日期在过去 30 年的历史数据**。曾经微软的 MSN 天气，没有广告，而且有一个很实用的功能，可以查看过去三十年，这一天的降水天数、最高或最低温度，可惜后来这些独特的功能都下架了。因此，我想拥有一个属于自己的天气助手，弥补常见的“中国天气网”、“墨迹天气”等 App 的不足，添加自己喜欢的卡片。

## 功能

- **中英文切换**：顶栏一键切换，选择本地记忆；城市名、天气描述、等级文案、空气质量口径均随语言联动。
- **自动定位**：打开网页自动获取用户位置并加载当地天气（需浏览器授权）；拒绝或不可用时回退北京并提示。顶栏 📍 按钮可随时重新定位。上次查看的位置会被记住，下次打开直接加载。城市名由 BigDataCloud 免费客户端接口反查。
- **城市搜索**：中英双语自动补全（Open-Meteo Geocoding），默认加载北京。
- **常见天气卡片**：
  - 当前天气主卡（温度、天气图标、体感温度、天气描述）
  - 湿度：270° 仪表弧（指针 + 分段着色）
  - 风力：风速等级 + MSN 风格刻度罗盘（风速、风向合并卡）
  - 气压 / 紫外线 / 空气质量：分段指示条
  - 空气质量：中文按中国国标（HJ 633-2012，含首要污染物），英文按 US AQI
  - 月相：八相 + 照亮比例 + SVG 月相图（本地推算）
  - 日出 / 日落：合并跨双列卡，底部太阳轨迹弧显示当日太阳位置
  - 未来 7 天预报（图标、最高/最低温、降水概率；卡片 / 列表双模式）
- **过去 30 年今日天气卡片**（取所选城市当地日期，统计该日期在过去 30 年的历史数据）：
  - 🌧️ 降水天数：今天这个日期在过去 30 年里有几天降水；冬季自动转为"降雪天数"，改按降雪统计
  - 🌧️ 最大降水：今天这个日期在过去 30 年的最大降水量 + 年份 + 日期
  - 🔥 最高温度：今天这个日期在过去 30 年的最高温度 + 年份 + 日期
  - ❄️ 最低温度：今天这个日期在过去 30 年的最低温度 + 年份 + 日期

## 数据源

全部来自 [Open-Meteo](https://open-meteo.com/)（免费、开源、无需 API key、已开放 CORS）：

| 用途 | 接口 |
| --- | --- |
| 城市搜索 | `geocoding-api.open-meteo.com` |
| 实时天气 + 7 天预报 | `api.open-meteo.com/v1/forecast` |
| 历史 30 年逐日数据 | `archive-api.open-meteo.com/v1/archive` |
| 空气质量（PM2.5/PM10/O₃/NO₂/SO₂/CO 浓度 + US AQI） | `air-quality-api.open-meteo.com/v1/air-quality` |

## 性能与缓存

- 30 年历史数据约 1 万条逐日记录，单次全量拉取约需 35 秒。本站将窗口拆成 **6 个 5 年分块并发拉取**，首载降至约 6 秒；之后原始数据写入 `localStorage` 并缓存 **30 天**，再次打开即时返回。
- 实时天气缓存 **10 分钟**、空气质量缓存 **30 分钟**，避免频繁请求。
- 天气与历史数据**并行拉取**，总耗时取两者较慢者（约 6 秒）。
- "今天"每换一天，前端从缓存数据重新过滤计算，无需重新拉取。

## 本地运行

无需构建步骤，任选其一：

```bash
# 方式一：Python 内置静态服务器
python3 -m http.server 8123
# 浏览器打开 http://localhost:8123

# 方式二：Node
npx serve .
```

也可以直接双击 `index.html` 用 `file://` 打开（API 允许跨域，可正常工作）。注意：浏览器定位（Geolocation）需要安全环境（HTTPS 或 localhost），`file://` 下多数浏览器会禁用定位，此时回退北京。

## 桌面应用（Tauri）

仓库同时包含一个 Tauri 2 桌面壳（`src-tauri/`）。前端**直接复用**本目录的 `index.html` / `app.js` / `style.css`（由 `copy-frontend.js` 拷入 `dist/` 嵌入安装包），网页版与桌面应用共用同一份代码，GitHub Pages 部署不受影响。

桌面特性：

- 独立窗口加载站点（Linux 用 WebKitGTK，Windows 用系统 WebView2，安装包很小）
- 系统托盘：左键单击恢复窗口，右键菜单含「显示主窗口 / 开机自启开关 / 退出」
- 托盘天气：托盘图标随当前天气变化（晴/夜晴/多云/阴/雾/雨/雪/雷暴 8 组图形，随昼夜与语言自动切换），鼠标悬停在图标上会弹出详情卡片（温度、体感、湿度、风力、气压、AQI、明日预报），点击卡片打开主窗口
- 关闭窗口最小化到托盘，常驻后台；退出请走托盘菜单

**Linux 本地开发**（Debian / Ubuntu / Kali）：

```bash
# 系统依赖 + Rust（rustc ≥ 1.77.5，apt 版或 rustup 均可）
sudo apt install rustc cargo libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev

npm install
npx tauri dev      # 开发运行
npx tauri build    # 产出 .deb / AppImage 于 src-tauri/target/release/bundle/
```

**Windows**：推荐直接在 [GitHub Releases](https://github.com/secnotes/weather/releases) 下载 `.msi` / `.exe` 安装包。推送 `v*` tag 会触发 GitHub Actions（`.github/workflows/tauri-release.yml`）自动构建 Windows 与 Linux 安装包并发布到 Releases。

**Windows 本地构建**：

1. 安装前置工具（一次即可）：
   - **Rust**：运行 [rustup-init.exe](https://rustup.rs)，默认选项安装（工具链 `stable-x86_64-pc-windows-msvc`）
   - **Visual Studio 2022 生成工具**：勾选「使用 C++ 的桌面开发」工作负载（需含 MSVC v143 与 Windows 11 SDK），约 3-4 GB
   - **Node.js** ≥ 18（仅用于 Tauri CLI）
   - WebView2 为 Win10/11 系统自带，无需单独安装
2. 构建：

   ```bash
   npm install
   npx tauri build
   ```

3. 产物位于（二选一安装）：
   - `src-tauri/target/release/bundle/msi/Weather_x.y.z_x64_en-US.msi`
   - `src-tauri/target/release/bundle/nsis/Weather_x.y.z_x64-setup.exe`

**国内网络提速**（直连慢/被拒时）：

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

注意事项：

- Linux 下 WebKitGTK 定位依赖 geoclue 服务，多数环境会失败——应用会沿用网页逻辑自动回退北京
- GNOME 默认不显示托盘图标，需安装 AppIndicator/KStatusNotifier 扩展；KDE / XFCE 原生支持
- 托盘悬停弹窗在三平台均可用（Linux 托盘经光标轮询检测悬停，略有延迟属正常）
- Windows 11 默认把新应用收进托盘溢出区，可在「任务栏设置 → 其他系统托盘图标」中开启常显，或把图标拖到任务栏
- 天气图标使用 emoji，Linux 需安装 `fonts-noto-color-emoji`

## 说明

- Open-Meteo 免费版非商业用途每日 10,000 次调用上限，个人使用绰绰有余。
- 历史归档数据可能滞后数天，最近几天的数据会随补全而更新。
- 风速为 10m 高度风速，温度为 2m 气温，降雪量为 cm，符合 WMO 标准。
