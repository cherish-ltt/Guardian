import { ref } from 'vue'
import { systemApi, type IpLocationItem } from '@/api'
import { toastError } from '@/composables/toast'

/* IP 属地现查：属地不入库，由用户点按钮查。
   结果按「原始 IP → 地名」缓存在页面内，切换分页不用重复请求；
   地名里没有「本机 / 内网」这类标记，本地网段同样按离线库的结论显示。 */
export function useIpLocation() {
  const locations = ref<Record<string, string>>({})
  const pending = ref(false)

  function locationOf(ip?: string) {
    if (!ip) return ''
    return locations.value[ip] ?? ''
  }

  function apply(items: IpLocationItem[]) {
    const next = { ...locations.value }
    for (const item of items) next[item.ip] = item.location
    locations.value = next
  }

  async function locate(ip?: string) {
    if (!ip || locationOf(ip)) return
    pending.value = true
    try {
      apply([await systemApi.ipLocations.locate(ip)])
    } catch (error) {
      toastError(error, '属地查询失败')
    } finally {
      pending.value = false
    }
  }

  /** 整页批量查询：只查还没有结果的地址。 */
  async function locateMany(ips: (string | undefined)[]) {
    const targets = [...new Set(ips.filter((ip): ip is string => Boolean(ip) && !locationOf(ip)))]
    if (!targets.length) return
    pending.value = true
    try {
      apply(await systemApi.ipLocations.locateMany(targets))
    } catch (error) {
      toastError(error, '整页属地查询失败')
    } finally {
      pending.value = false
    }
  }

  return { locations, pending, locationOf, locate, locateMany }
}
