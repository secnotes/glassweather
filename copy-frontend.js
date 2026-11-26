#!/usr/bin/env node
/* 把网页版前端（仓库根目录三个文件）拷进 dist/ 供 Tauri 嵌入。
   网页版继续由 GitHub Pages 从根目录部署，桌面应用与网页共用同一份源码。
   跨平台（Linux/macOS/Windows CI 通用），无需 shell 命令。 */
'use strict';
const fs = require('fs');
const path = require('path');

const FILES = ['index.html', 'app.js', 'style.css', 'tray-popup.html', 'tray-bar.html', 'tray-deskband.html', 'city-zh.json'];
const root = path.join(__dirname);
const dest = path.join(root, 'dist');

fs.mkdirSync(dest, { recursive: true });
for (const f of FILES) {
  fs.copyFileSync(path.join(root, f), path.join(dest, f));
  console.log(`copied ${f} -> dist/`);
}
