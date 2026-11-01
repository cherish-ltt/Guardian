import { strict as assert } from 'node:assert'
import { test } from 'node:test'
import { menuPages } from '../menu-pages.ts'

test('collects authorized pages at any depth and skips unsupported nodes', () => {
  interface Node { name: string; supported?: boolean; children?: Node[] }
  const tree: Node[] = [
    { name: 'one', supported: true },
    { name: 'directory', children: [
      { name: 'two', supported: true },
      { name: 'nested', children: [{ name: 'three', supported: true }, { name: 'unknown' }] },
    ] },
  ]
  assert.deepEqual(menuPages(tree, node => node.supported === true).map(node => node.name), ['one', 'two', 'three'])
})
