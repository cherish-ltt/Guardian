export function menuPages<T extends { children?: T[] }>(nodes: T[], isPage: (node: T) => boolean): T[] {
  return nodes.flatMap(node => [...(isPage(node) ? [node] : []), ...menuPages(node.children ?? [], isPage)])
}
