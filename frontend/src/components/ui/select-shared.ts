/**
 * 下拉搜索框的开关：单选（Select）与多选（MultiCheck）共用。
 * auto 时选项数达到阈值才显示搜索框，选项少时省掉输入行；
 * true / false 由调用方显式覆盖，用于必须能搜或刻意不要搜索的场合。
 */
export const SELECT_SEARCH_THRESHOLD = 8

export function searchEnabled(filterable: boolean | 'auto', count: number) {
  return filterable === 'auto' ? count >= SELECT_SEARCH_THRESHOLD : filterable
}
