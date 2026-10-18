import test from 'node:test'
import assert from 'node:assert/strict'
import { parse, compileScript, compileTemplate } from '@vue/compiler-sfc'
import { transformSync } from 'esbuild'
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { createSSRApp, h } from 'vue'
import { renderToString } from '@vue/server-renderer'

/**
 * 单文件组件的最小渲染装置。
 *
 * 项目不引 vitest / jsdom：组件测试用 node:test + @vue/compiler-sfc 现场编译 SFC，
 * 用 esbuild 去掉 TS 语法，再用 @vue/server-renderer 输出 HTML 断言。
 * 三者都已在依赖树里（esbuild 是 vite 的依赖），不新增依赖，也不需要浏览器。
 *
 * 限制：被测组件不能 import 其他 SFC。`@/` 别名与 `.vue` 导入在 Node 里没有打包器
 * 解析，所以断言只落在无子组件的纯展示控件上。
 */
const componentsDir = new URL('../', import.meta.url)
const buildDir = new URL('../../.sfc-build/', import.meta.url)

function toSourcePath(relativePath: string) {
  return new URL(relativePath, componentsDir)
}

function stripTypes(code: string, name: string) {
  return transformSync(code, {
    loader: 'ts',
    format: 'esm',
    target: 'es2022',
    sourcefile: name,
  }).code
}

/** 编译并渲染一个 SFC，返回服务端渲染出的 HTML 字符串。 */
export async function renderComponent(
  relativePath: string,
  props: Record<string, unknown> = {},
  slots: Record<string, () => string> = {},
) {
  const filename = toSourcePath(relativePath).pathname
  const { descriptor, errors } = parse(readFileSync(filename, 'utf8'), { filename })
  assert.deepEqual(errors, [], `${relativePath} 解析失败`)

  const script = compileScript(descriptor, { id: filename })
  const templateSource = descriptor.template?.content
  assert.ok(templateSource, `${relativePath} 没有 template`)
  const template = compileTemplate({
    source: templateSource,
    filename,
    id: filename,
    compilerOptions: { bindingMetadata: script.bindings },
  })
  assert.deepEqual(template.errors, [], `${relativePath} 模板编译失败`)

  // 编译产物是 TS 源码，Node 不能直接执行；转译成 ESM 落到临时目录再 import。
  mkdirSync(buildDir, { recursive: true })
  const stamp = `${relativePath.replace(/\W+/g, '_')}-${Math.random().toString(36).slice(2)}`
  const scriptPath = join(buildDir.pathname, `${stamp}.script.mjs`)
  const templatePath = join(buildDir.pathname, `${stamp}.template.mjs`)
  writeFileSync(scriptPath, stripTypes(script.content, `${relativePath}.script.ts`))
  writeFileSync(templatePath, stripTypes(template.code, `${relativePath}.template.ts`))

  try {
    const scriptModule = await import(pathToFileURL(scriptPath).href)
    const templateModule = await import(pathToFileURL(templatePath).href)
    const component = { ...scriptModule.default, render: templateModule.render }

    return await renderToString(createSSRApp({ render: () => h(component, props, slots) }))
  } finally {
    rmSync(scriptPath, { force: true })
    rmSync(templatePath, { force: true })
  }
}

test('renderComponent 能渲染纯展示控件', async () => {
  const html = await renderComponent(
    'ui/Tag.vue',
    { tone: 'ok', dot: true },
    { default: () => '正常' },
  )

  assert.match(html, /class="tag tone-ok"/)
  assert.match(html, /class="tag-dot"/)
  assert.match(html, /正常/)
})
