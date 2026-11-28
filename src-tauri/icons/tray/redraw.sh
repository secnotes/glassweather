#!/usr/bin/env bash
# 托盘图标针对性重画（2026-11）：16px 托盘下"看不清"的 5 组图形重绘。
# 病灶与对策：
#   cloudy 浅灰云在浅色任务栏上几乎隐形 -> 加深云色 + 描边加重
#   fog    雾线与云同色系糊成一团        -> 雾线加白加粗、砍一条减密度
#   partly 太阳与云互相挤压变小          -> 太阳加大、云压小，主次分明
#   snow   六角雪花笔画太细缩成灰点      -> 改实心雪粒（圆点，粗体感强）
#   night  弯月轮廓在浅色任务栏发虚      -> 描边加重
# 其余（clear/rain/thunder）对比度尚可，不动。
# 运行：bash redraw.sh   （依赖 node + @resvg/resvg-js，见 render.js）
set -euo pipefail
cd "$(dirname "$0")"

SUN_FILL='#FFC93C'
CLOUD_FILL='#EAF0F7'
CLOUD_DEEP='#BCC8D6'    # 阴天云：比原 D3DCE8 深一档
RAIN='#5EB1EF'
SNOW='#BFE3FF'          # 雪粒：加亮，与云分层
BOLT='#FFD60A'
MOON='#F5E6B8'
FOG='#C9DCED'           # 雾线：比云浅、加白

head() { cat <<EOF
<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128">
EOF
}
foot() { echo '</svg>'; }

# 云：描边加重（6.5，托盘缩放后仍可见轮廓）
cloud() { # $1 fill
  cat <<EOF
<g fill="$1" stroke="rgba(22,34,46,0.65)" stroke-width="6.5" stroke-linejoin="round">
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

# 太阳（圆心 0,0，光芒 9px）
sun() { # $1 cx $2 cy $3 scale
  cat <<EOF
<g transform="translate($1,$2) scale($3)" stroke="$SUN_FILL" stroke-width="9" stroke-linecap="round">
  $(for a in 0 45 90 135 180 225 270 315; do
      echo "<line x1='0' y1='-30' x2='0' y2='-42' transform='rotate($a)'/>"; done)
</g>
<circle cx="$1" cy="$2" r="21" fill="$SUN_FILL"/>
EOF
}

# 3. 多云（晴间多云）：太阳加大居左上、云压小右下——主次分明
{ head; cat <<EOF
<g transform="translate(-4,2)">$(sun 44 42 1)</g>
<g transform="translate(14,26) scale(0.78)">$(cloud "$CLOUD_FILL")</g>
EOF
foot; } > partly.svg

# 4. 阴：云色加深
{ head; cat <<EOF
<g transform="translate(0,6)">$(cloud "$CLOUD_DEEP")</g>
EOF
foot; } > cloudy.svg

# 5. 雾：云压小上移，雾线加白加粗、三条减两条
{ head; cat <<EOF
<g transform="translate(4,-20) scale(0.82)">$(cloud "$CLOUD_DEEP")</g>
<g stroke="$FOG" stroke-width="10" stroke-linecap="round">
  <line x1="30" y1="98" x2="98" y2="98"/>
  <line x1="42" y1="116" x2="96" y2="116"/>
</g>
EOF
foot; } > fog.svg

# 7. 雪：云 + 实心雪粒（粗圆点，16px 下仍是明确白点）
{ head; cat <<EOF
<g transform="translate(0,-14)">$(cloud "$CLOUD_FILL")</g>
<g fill="$SNOW">
  <circle cx="48" cy="108" r="7.5"/>
  <circle cx="67" cy="116" r="7.5"/>
  <circle cx="86" cy="108" r="7.5"/>
</g>
EOF
foot; } > snow.svg

# 2. 晴夜：弯月描边加重（浅色任务栏上轮廓不虚）
{ head; cat <<EOF
<path d="M94 40 A40 40 0 1 0 94 96 A31 31 0 1 1 94 40 Z" fill="$MOON"
      stroke="rgba(22,34,46,0.65)" stroke-width="6" stroke-linejoin="round"/>
<g fill="#FFF3C4">
  <path d="M40 30 l3.2 7.8 7.8 3.2 -7.8 3.2 -3.2 7.8 -3.2 -7.8 -7.8 -3.2 7.8 -3.2 z"/>
  <path d="M104 86 l2.2 5.4 5.4 2.2 -5.4 2.2 -2.2 5.4 -2.2 -5.4 -5.4 -2.2 5.4 -2.2 z"/>
</g>
EOF
foot; } > night.svg

for name in partly cloudy fog snow night; do
  node render.js "${name}.svg" "${name}.png" 128
  echo "redrawn ${name}.png"
done
