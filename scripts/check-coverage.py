"""校验 cargo llvm-cov 的 JSON 汇总：全局行覆盖率不得低于 80%。

门槛来自 backend/AGENTS.md 第 10.1 节。口径是全局行覆盖率，不按文件考核。
"""

from __future__ import annotations

import json
import sys

THRESHOLD = 80.0


def main(path: str) -> int:
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)

    lines = data["data"][0]["totals"]["lines"]
    percent = float(lines["percent"])
    print(f"行覆盖率 {percent:.2f}%（{lines['covered']}/{lines['count']}）")

    if percent < THRESHOLD:
        print(f"行覆盖率低于 {THRESHOLD:.0f}% 门槛", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "coverage.json"))
