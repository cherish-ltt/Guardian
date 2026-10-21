# 未发布变更

本文件累积下一次发布要带上的变更，发版时并入 `vX.Y.Z.md` 后清空（规则见仓库根 `AGENTS.md`）。
小节结构与正式发布说明一致，按「后端」「前端」分别罗列，没有改动的部分保留小节标题并写「无」。

## 后端

- 无。

## 前端

- 用户表单的「岗位」「角色」多选下拉改为与「所属部门」同一种下拉：浮层 Teleport 到 body、层级随所在弹窗、
  空间不足向上展开，并补齐搜索、方向键 / 回车 / Esc 与 `role="listbox"` 无障碍语义。
- 下拉搜索框改为按选项数自动开启（单选与多选共用 `src/components/ui/select-shared.ts` 的阈值），
  需要强制开启或关闭时显式传 `filterable`；浮层定位逻辑抽到 `src/composables/popover.ts`，由 `Select` 与 `MultiCheck` 共用。
