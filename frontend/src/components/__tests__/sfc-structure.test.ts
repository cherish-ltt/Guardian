import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

/**
 * SFC 结构守卫。
 *
 * `import` 落在 `<script setup>` 外面时，SFC 编译器不报错，但组件不会注册：
 * 模板里的 `<Modal>` 被当成未知原生标签原样渲染到页面上，弹窗内容就漏在页面底部。
 * 这类错误 type-check 与 lint 都发现不了，只能靠结构断言拦住。
 */
const srcDir = new URL('../../', import.meta.url).pathname

function vueFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) return vueFiles(full)
    return full.endsWith('.vue') ? [full] : []
  })
}

test('每个 .vue 的 import 都在 <script> 块内', () => {
  const offenders: string[] = []

  for (const file of vueFiles(srcDir)) {
    const lines = readFileSync(file, 'utf8').split('\n')
    let inScript = false
    lines.forEach((line, index) => {
      const trimmed = line.trim()
      if (trimmed.startsWith('<script')) inScript = true
      else if (trimmed.startsWith('</script')) inScript = false
      else if (/^(import|export)\s/.test(trimmed) && !inScript) {
        offenders.push(`${file.slice(srcDir.length)}:${index + 1}`)
      }
    })
  }

  assert.deepEqual(offenders, [], `这些文件的 import 落在 <script> 外：\n${offenders.join('\n')}`)
})

test('每个 .vue 都有 <script setup> 或 <script> 块', () => {
  const offenders = vueFiles(srcDir).filter((file) => !/<script[\s>]/.test(readFileSync(file, 'utf8')))

  assert.deepEqual(offenders.map((file) => file.slice(srcDir.length)), [])
})
