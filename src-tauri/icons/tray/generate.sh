#!/usr/bin/env bash
# 托盘天气图标（8 组天气类型，不含温度；温度在悬停弹窗中显示）。
# 源文件为同目录下的 SVG（便于日后调整），用 ImageMagick 栅格化为 128x128 PNG，
# Rust 侧 include_bytes! 嵌入。重新生成：./generate.sh
set -euo pipefail
cd "$(dirname "$0")"

SUN_FILL='#FFC93C'
CLOUD_FILL='#EAF0F7'
CLOUD_DARK='#D3DCE8'
RAIN='#5EB1EF'
SNOW='#85BCED'
BOLT='#FFD60A'
MOON='#F5E6B8'
FOG='#9DB6CC'

# 通用云朵（viewBox 128）：重叠圆 + 底部圆角矩形，填充后呈蓬松云形。
# 两遍绘制：先画带描边的联合体，再用同色无描边联合体覆盖内部——
# 内圈描边接缝被第二遍不透明填充盖住，只留外轮廓描边，保证白色云在浅色任务栏上也可见。
cloud() { # $1 = fill color
  cat <<EOF
<g fill="$1" stroke="rgba(22,34,46,0.55)" stroke-width="5" stroke-linejoin="round">
  <rect x="34" y="86" width="68" height="26" rx="13"/>
  <circle cx="52" cy="84" r="22"/>
  <circle cx="78" cy="68" r="30"/>
  <circle cx="102" cy="88" r="19"/>
</g>
<g fill="$1">
  <rect x="34" y="86" width="68" height="26" rx="13"/>
  <circle cx="52" cy="84" r="22"/>
  <circle cx="78" cy="68" r="30"/>
  <circle cx="102" cy="88" r="19"/>
</g>
EOF
}

# 太阳（圆心 0,0，半径 r18，光芒 9px）
sun() { # $1 = cx  $2 = cy  $3 = ray extra scale(1 或 0.7)
  local cx=$1 cy=$2 s=$3
  cat <<EOF
<g transform="translate($cx,$cy) scale($s)" stroke="$SUN_FILL" stroke-width="8" stroke-linecap="round">
  $(for a in 0 45 90 135 180 225 270 315; do
      echo "<line x1='0' y1='-30' x2='0' y2='-42' transform='rotate($a)'/>"; done)
</g>
<circle cx="$cx" cy="$cy" r="20" fill="$SUN_FILL"/>
EOF
}

head() { cat <<EOF
<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
EOF
}
foot() { echo '</svg>'; }

# 1. 晴（白天）
{ head; sun 64 64 1; foot; } > clear.svg

# 2. 晴（夜间）：弯月 + 小星
{ head; cat <<EOF
<path d="M94 40 A40 40 0 1 0 94 96 A31 31 0 1 1 94 40 Z" fill="$MOON"
      stroke="rgba(22,34,46,0.45)" stroke-width="4" stroke-linejoin="round"/>
<g fill="#FFF3C4">
  <path d="M40 30 l3.2 7.8 7.8 3.2 -7.8 3.2 -3.2 7.8 -3.2 -7.8 -7.8 -3.2 7.8 -3.2 z"/>
  <path d="M104 86 l2.2 5.4 5.4 2.2 -5.4 2.2 -2.2 5.4 -2.2 -5.4 -5.4 -2.2 5.4 -2.2 z"/>
</g>
EOF
foot; } > night.svg

# 3. 多云（晴间多云）：小太阳（左上）+ 云（右下）
{ head; cat <<EOF
<g transform="translate(2,6) scale(0.9)">$(sun 42 40 0.85)</g>
<g transform="translate(10,22) scale(0.86)">$(cloud "$CLOUD_FILL")</g>
EOF
foot; } > partly.svg

# 4. 阴
{ head; cat <<EOF
<g transform="translate(0,6)">$(cloud "$CLOUD_DARK")</g>
EOF
foot; } > cloudy.svg

# 5. 雾：小云 + 三条雾线
{ head; cat <<EOF
<g transform="translate(0,-14) scale(0.92)">$(cloud "$CLOUD_DARK")</g>
<g stroke="$FOG" stroke-width="7" stroke-linecap="round">
  <line x1="34" y1="92" x2="92" y2="92"/>
  <line x1="44" y1="105" x2="100" y2="105"/>
  <line x1="34" y1="118" x2="86" y2="118"/>
</g>
EOF
foot; } > fog.svg

# 6. 雨：云 + 三道雨线
{ head; cat <<EOF
<g transform="translate(0,-14)">$(cloud "$CLOUD_FILL")</g>
<g stroke="$RAIN" stroke-width="8" stroke-linecap="round">
  <line x1="50" y1="100" x2="43" y2="118"/>
  <line x1="67" y1="100" x2="60" y2="118"/>
  <line x1="84" y1="100" x2="77" y2="118"/>
</g>
EOF
foot; } > rain.svg

# 7. 雪：云 + 三枚六角雪花
flake() { # $1 cx  $2 cy
  cat <<EOF
<g transform="translate($1,$2)" stroke="$SNOW" stroke-width="5" stroke-linecap="round">
  $(for a in 0 60 120; do echo "<line x1='0' y1='-8' x2='0' y2='8' transform='rotate($a)'/>"; done)
</g>
EOF
}
{ head; cat <<EOF
<g transform="translate(0,-14)">$(cloud "$CLOUD_FILL")</g>
$(flake 49 110)
$(flake 66 117)
$(flake 83 110)
EOF
foot; } > snow.svg

# 8. 雷暴：云 + 闪电
{ head; cat <<EOF
<g transform="translate(0,-12)">$(cloud "$CLOUD_DARK")</g>
<path d="M70 90 L46 120 L62 120 L55 136 L86 100 L70 100 L78 90 Z" fill="$BOLT"
      stroke="#E8A90B" stroke-width="2" stroke-linejoin="round"/>
EOF
foot; } > thunder.svg

# 栅格化优先 rsvg-convert，其次 node + @resvg/resvg-js（见 render.js 头注释）。
# ImageMagick 内置 MSVG 渲染器会把嵌套 transform / stroke 画错，不用它。
render_png() { # $1 name
  if command -v rsvg-convert >/dev/null 2>&1; then
    rsvg-convert -w 128 -h 128 "${1}.svg" -o "${1}.png"
  elif NODE_PATH="${NODE_PATH:-}:$(dirname "$0")/node_modules" node -e "require('@resvg/resvg-js')" >/dev/null 2>&1; then
    NODE_PATH="${NODE_PATH:-}:$(dirname "$0")/node_modules" node "$(dirname "$0")/render.js" "${1}.svg" "${1}.png" 128
  else
    echo "需要 rsvg-convert (apt install librsvg2-bin) 或 node 依赖 @resvg/resvg-js" >&2
    exit 1
  fi
}

for name in clear night partly cloudy fog rain snow thunder; do
  render_png "$name"
  echo "rendered ${name}.png"
done
