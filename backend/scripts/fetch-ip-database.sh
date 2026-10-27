#!/usr/bin/env sh
# 下载 ip2region 离线地名库到 data/。库文件不入库（见 .gitignore 与 docs/DEPLOYMENT.md），
# 克隆后跑一次即可；上游每周更新，需要最新地名时重跑并重启服务。
#
# 用法：
#   backend/scripts/fetch-ip-database.sh            # 从 GitHub 直连下载
#   IP_DB_MIRROR=https://proxyhub.520350.xyz/ backend/scripts/fetch-ip-database.sh
#
# 镜像前缀拼在原始地址前（如 `https://proxyhub.520350.xyz/https://raw.githubusercontent.com/...`）。
set -eu

MIRROR="${IP_DB_MIRROR:-}"
BASE="https://raw.githubusercontent.com/lionsoul2014/ip2region/master/data"
TARGET="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)/data"

mkdir -p "$TARGET"

for name in ip2region_v4.xdb ip2region_v6.xdb; do
    echo "downloading ${name}"
    curl -fsSL --retry 5 --retry-delay 2 -o "${TARGET}/${name}.part" "${MIRROR}${BASE}/${name}"
    mv "${TARGET}/${name}.part" "${TARGET}/${name}"
done

ls -l "${TARGET}"/ip2region_v*.xdb
