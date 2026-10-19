"""校验 cargo llvm-cov 的 JSON 汇总：全局行覆盖率不得低于 80%。

门槛来自 backend/AGENTS.md 第 10.1 节。口径是全局行覆盖率，不按文件考核。
"""

from __future__ import annotations

import json
import sys

THRESHOLD = 80.0


def fail(message: str) -> int:
    print(message, file=sys.stderr)
    return 1


def main(path: str) -> int:
    try:
        with open(path, encoding="utf-8") as handle:
            data = json.load(handle)
    except OSError as error:
        return fail(f"读不到覆盖率报告 {path}：{error}")
    except json.JSONDecodeError as error:
        return fail(f"覆盖率报告 {path} 不是合法 JSON：{error}")

    try:
        lines = data["data"][0]["totals"]["lines"]
        percent = float(lines["percent"])
        covered = lines["covered"]
        count = lines["count"]
    except (KeyError, IndexError, TypeError) as error:
        return fail(f"覆盖率报告 {path} 结构不符合 llvm-cov 输出：{error!r}")

    print(f"行覆盖率 {percent:.2f}%（{covered}/{count}）")

    if percent < THRESHOLD:
        return fail(f"行覆盖率低于 {THRESHOLD:.0f}% 门槛")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "coverage.json"))
