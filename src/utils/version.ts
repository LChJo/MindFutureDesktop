/**
 * 语义化版本号比较工具
 */

/**
 * 比较两个语义化版本号
 * @param a 版本号 a
 * @param b 版本号 b
 * @returns 1 如果 a > b, -1 如果 a < b, 0 如果相等
 */
export function compareVersions(a: string, b: string): number {
  // 处理空值
  if (!a && !b) return 0
  if (!a) return -1
  if (!b) return 1

  // 分离主版本和预发布标签 (如 1.0.0-beta.1)
  const [aMain, aPre] = a.split('-')
  const [bMain, bPre] = b.split('-')

  // 比较主版本号
  const aParts = aMain.split('.').map(Number)
  const bParts = bMain.split('.').map(Number)
  const len = Math.max(aParts.length, bParts.length)

  for (let i = 0; i < len; i++) {
    const aNum = aParts[i] || 0
    const bNum = bParts[i] || 0
    if (aNum > bNum) return 1
    if (aNum < bNum) return -1
  }

  // 主版本相同时，比较预发布标签
  // 有预发布标签的版本 < 没有预发布标签的版本 (1.0.0-beta < 1.0.0)
  if (aPre && !bPre) return -1
  if (!aPre && bPre) return 1

  return 0
}

/**
 * 检查版本 a 是否比版本 b 更新
 * @param a 新版本
 * @param b 旧版本
 * @returns true 如果 a > b
 */
export function isNewerVersion(a: string, b: string): boolean {
  return compareVersions(a, b) > 0
}
