#!/usr/bin/env node
// SVG -> PNG 栅格化（托盘图标重新生成时用；生成结果 *.png 已提交并被 Rust include_bytes! 嵌入，
// 普通构建/CI 不需要本脚本）。
// 用法：node render.js name.svg name.png [size=128]
// 依赖：npm i @resvg/resvg-js  （也可用 rsvg-convert，见 generate.sh）
// 不用 ImageMagick 自带的 MSVG 渲染器：它对嵌套 transform / stroke 继承支持有 bug。
'use strict';
const fs = require('fs');
const { Resvg } = require('@resvg/resvg-js');

const [, , inSvg, outPng, sizeArg] = process.argv;
const size = Number(sizeArg) || 128;
const svg = fs.readFileSync(inSvg);
const png = new Resvg(svg, { fitTo: { mode: 'width', value: size } })
  .render()
  .asPng();
fs.writeFileSync(outPng, png);
