/* 登录热力格：把后端稀疏格子补成 7 × 24 网格，并按分位分 0–4 级（纯函数） */
export interface HeatCell { dow: number; hour: number; total: number; failed: number }
export interface HeatGridCell extends HeatCell { level: 0 | 1 | 2 | 3 | 4; failRate: number; risky: boolean }

export function buildHeatGrid(cells: HeatCell[]) {
  const grid: HeatGridCell[][] = Array.from({ length: 7 }, (_, dow) =>
    Array.from({ length: 24 }, (_, hour) => ({ dow, hour, total: 0, failed: 0, level: 0, failRate: 0, risky: false })))
  for (const c of cells) {
    if (c.dow < 0 || c.dow > 6 || c.hour < 0 || c.hour > 23) continue
    const cell = grid[c.dow][c.hour]
    cell.total += c.total
    cell.failed += c.failed
  }
  const totals = grid.flat().map(c => c.total).filter(v => v > 0).sort((a, b) => a - b)
  const q = (p: number) => totals.length ? totals[Math.floor(p * (totals.length - 1))] : 0
  const cuts = [q(0.25), q(0.5), q(0.75)]
  let peak: HeatGridCell | null = null
  for (const cell of grid.flat()) {
    cell.failRate = cell.total ? cell.failed / cell.total : 0
    cell.risky = cell.failed >= 2 && cell.failRate >= 0.3
    cell.level = cell.total === 0 ? 0 : cell.total <= cuts[0] ? 1 : cell.total <= cuts[1] ? 2 : cell.total <= cuts[2] ? 3 : 4
    if (!peak || cell.total > peak.total) peak = cell
  }
  const total = totals.reduce((a, b) => a + b, 0)
  return { grid, peak: peak && peak.total ? peak : null, total }
}
