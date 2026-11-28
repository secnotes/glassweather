# GlassWeather（仓库目录名 weather）项目指引

> 双形态天气应用：纯前端静态网页（中英双语）+ Tauri 2 桌面壳，共用同一份前端源码。
> 招牌功能：任意城市「今天这个日期在过去 30 年的历史极值」（复刻已下架的 MSN 天气功能）。

## 项目结构（全部核心文件在仓库根目录，无 src 目录）

```
index.html          # 网页主骨架（卡片布局 + data-i18n 标记）
app.js              # 全部前端逻辑（约 2200 行，单文件零依赖 vanilla JS）
style.css           # 全部样式
city-zh.json        # 中文城市搜索本地库（约 3250 条，阿里 DataV Adcode 行政区划，
                    #   含 c 短名 / p 上级 / f 全名 / lat / lon；网页版与桌面版共用）
tray-popup.html     # 桌面版托盘悬停详情弹窗卡片
tray-bar.html       # 桌面版 Windows 悬浮天气条（任务栏上方右下角常驻小窗）
tray-deskband.html  # 桌面版 Win11 任务栏天气（内嵌任务栏，字色随 Windows 系统主题）
copy-frontend.js    # 构建前把上述文件拷入 dist/ 供 Tauri 嵌入（网页版不受影响）
src-tauri/          # Rust 壳层（main.rs 一个文件 + Cargo.toml + tauri.conf.json
                    #   + tauri.windows.conf.json（Windows 专属 DLL 资源）+ capabilities/）
.github/workflows/tauri-release.yml   # 推 v* tag 触发 CI 构建发布
```

## 数据源（全部免 key、开放 CORS）

- **Open-Meteo 四接口**：geocoding（城市搜索/坐标反查）、forecast（实时+7天）、archive（30 年历史）、air-quality
- **city-zh.json**（DataV Adcode）：中文搜索独占使用 —— 含汉字的查询只查本地库（不再请求远程），英文/拼音走 Open-Meteo
- BigDataCloud 仅用于定位反查地名（桌面版有 referer 白名单问题会显示「当前位置」，属已知待修）

## 桌面版架构要点（改 main.rs 前必读）

- **三入口天气展示**：系统托盘（原生 tooltip）、悬浮天气条、Win11 deskband 任务栏天气（tauri-plugin-deskband11lib，仅 Win11）
- **弹窗显隐**：150ms 光标轮询驱动（`poll_hover`），**不依赖 tray-icon Enter/Leave**（其 Leave 有已知缺陷）；含 450ms 悬停延迟与全屏抑制（`foreground_is_fullscreen`）
- **Win32 原生调用约定**：弹窗显示/隐藏走 `SW_SHOWNOACTIVATE`/`SW_HIDE`（对称，绕过框架可见状态跟踪）；窗口圆角用 DWM `DWMWCP_ROUND`；后台线程**只做纯 Win32 调用**，不碰 Tauri 异步 API
- **Tauri 命令**：`set_tray_lang` / `set_tray_weather` / `popup_clicked`；前端桥接双通道（invoke + Rust 2s 轮询 eval 兜底）
- **deskband 位置**：托盘菜单「任务栏天气位置」4 档可选（TaskbarContentPlacement），持久化 deskband-place.cfg；运行中切换需**先 stop → 延迟 600ms → 主线程 start**（窗口销毁异步，立即重建撞 label）
- **配置文件**（`%APPDATA%\com.secnotes.weather\`）：bar.cfg（悬浮条开关）、deskband.cfg（任务栏天气开关）、deskband-place.cfg，格式 `on`/`off`
- identifier 固定 `com.secnotes.weather`（勿改，改则用户配置失联）；Cargo 包名 `glassweather`（产物 exe 名）

## 前端约定

- **零构建零依赖**：vanilla JS，无 npm 前端依赖（node_modules 仅 Tauri CLI）；网页版直接 file:// 或静态服务器可跑
- **i18n**：`I18N` 字典 + `data-i18n` 标记；托盘菜单文案由 Rust 侧 `tray_texts()` 双语维护
- **缓存**：`weather:` 前缀 localStorage；天气 10 分钟 / AQI 30 分钟 / 历史 30 天；静默刷新（visibilitychange + 桌面版 10 分钟定时器）不请求历史归档（immutable 数据）
- **改 app.js 后必须立即语法检查**：`node -e "new Function(require('fs').readFileSync('app.js','utf8'))"` —— 历史上一次 Edit 碰坏多行注释头导致整个 app.js 失效且带病发版（v0.2.1 事故），此为硬性规则
- **托盘 HTML 页面**点击监听绑在 `document`（不是 body）；读 localStorage('weather:tray') 渲染

## 构建与发布流程（Windows）

1. 前置：rustup（stable-msvc）+ VS2022 Build Tools（C++ 负载）+ Node ≥18；`~/.cargo/config.toml` 已配中科大镜像
2. `npx tauri build` —— **构建前先 `taskkill //IM glassweather.exe //F`**（应用常驻托盘会锁产物文件；NSIS 写 exe 偶发"另一个程序正在使用"是 Defender 扫描锁，等 15 秒重试即可）
3. 产物：`bundle/msi/*.msi`、`bundle/nsis/*-setup.exe`、裸 exe `target/release/glassweather.exe`（旁需 deskband11lib.dll）
4. **发版**：改版本号**四处**（Cargo.toml〔真相源〕/ tauri.conf.json / package.json / Cargo.lock 由 cargo build 自动更新）→ `chore: bump version to X.Y.Z` → `git tag vX.Y.Z` → push tag → CI（check-version 校验 tag↔Cargo.toml，不一致即失败）→ 草稿 Release（6 附件：msi/setup/portable zip/deb/rpm/AppImage）→ 手动替换 body（releaseBody 必须**纯 ASCII**，中文会导致 "Request body length" 错误）→ Publish
5. 版本号语义：0.x 阶段，feat 级 → minor，fix/style 级攒批 → patch

## Commit 规范（严格执行）

- **前缀**：`feat:` / `fix:` / `docs:` / `chore:` / `style:`，主题行首词小写（如 `feat: add xxx`）
- **格式**：英文主题 + 英文正文（要点列表）+ `中文说明：` 段（对应中文要点）+ 结尾署名行：
  `Co-Authored-By: Claude Code <noreply@anthropic.com>`
- 历史 8 条曾批量补过前缀（git filter-branch），**不要**重写已推送的历史；用户偶尔会自己微调 README 后让你一并提交

## 用户协作习惯

- 用户中文交流；验证习惯是装新包实测（构建产物在 `src-tauri/target/release/bundle/`）
- 提交前用户可能自己改过文件 —— `git status` 先看有没有用户未暂存的改动
- 「先别 push」是常见指令；push/发 tag 通常明确授权后才执行
- 微调类需求（字号、间距、文案）高频，改动小、期望快
- 构建放后台（run_in_background），完成通知后汇报；不要在等待时轮询

## 常见陷阱清单（历史踩坑沉淀）

1. 改 app.js → 必跑语法检查（见上，v0.2.1 事故根因）
2. releaseBody 中文 → CI 创建 Release 失败（多字节 content-length 不匹配）
3. 构建被锁 → 先杀 glassweather.exe；偶发 Defender 锁等 15s 重试
4. deskband 停后立即重启 → 撞 window label，需 600ms 延迟
5. tauri.conf.json 的 bundle.resources 声明平台专属文件 → Linux 构建失败，须拆到 tauri.windows.conf.json
6. Cargo.lock 的包名改动 → 需 `cargo build` 让 lockfile 重写（`cargo update -p` 可能查不到旧名）
7. setup 闭包里 `&mut App` 传给要 `&AppHandle` 的函数 → 先 `app.handle().clone()` 转换
8. TaskbarWindowHandle 非 Send → thread_local 存储，启停都在主线程
