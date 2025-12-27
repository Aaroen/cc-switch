/**
 * 状态样式工具函数
 * 根据 HTTP 状态码和请求状态返回对应的样式类
 */

// 请求状态类型（前端统一使用）
export type RequestStatus = 'pending' | 'completed'

// 状态色调类型（新增 pending）
type StatusTone = 'success' | 'redirect' | 'auth' | 'error' | 'info' | 'pending'

// Badge 样式映射（新增 pending 样式）
const badgeClassByTone: Record<StatusTone, string> = {
  success: 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400',
  redirect: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
  auth: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300',
  error: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
  info: 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300',
  pending: 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400 animate-request-pulse'  // 新增
}

// 点状指示器样式映射（新增 pending 样式）
const dotClassByTone: Record<StatusTone, string> = {
  success: 'bg-green-500',
  redirect: 'bg-blue-500',
  auth: 'bg-yellow-500',
  error: 'bg-red-500',
  info: 'bg-gray-400',
  pending: 'bg-gray-400 animate-request-pulse'  // 新增
}

// 容器背景色映射（新增 pending 样式）
const containerClassByTone: Record<StatusTone, string> = {
  success: 'bg-gray-50 dark:bg-gray-700',
  redirect: 'bg-blue-50 dark:bg-blue-900/20',
  auth: 'bg-yellow-50 dark:bg-yellow-900/20',
  error: 'bg-red-50 dark:bg-red-900/20',
  info: 'bg-gray-50 dark:bg-gray-700',
  pending: 'bg-gray-50 dark:bg-gray-700'  // 新增（不需要动画，避免闪烁）
}

// 特殊处理的认证状态码
const AUTH_STATUS_CODES = new Set([401, 407, 511])

/**
 * 获取状态色调（新增 status 参数）
 */
const getStatusTone = (statusCode?: unknown, status?: RequestStatus): StatusTone => {
  // 优先检查请求状态
  if (status === 'pending') return 'pending'

  // 如果状态码不是数字，返回 info
  if (typeof statusCode !== 'number') return 'info'

  // 根据状态码分类
  if (statusCode >= 200 && statusCode < 300) return 'success'
  if (statusCode >= 300 && statusCode < 400) return 'redirect'
  if (AUTH_STATUS_CODES.has(statusCode)) return 'auth'
  if (statusCode >= 400) return 'error'

  return 'info'
}

/**
 * 检查是否为错误状态
 */
export const isErrorStatus = (statusCode?: unknown): boolean => {
  return typeof statusCode === 'number' && statusCode >= 400
}

/**
 * 获取状态码 Badge 样式类（新增 status 参数）
 */
export const getStatusBadgeClass = (statusCode?: unknown, status?: RequestStatus): string => {
  return badgeClassByTone[getStatusTone(statusCode, status)]
}

/**
 * 获取状态点指示器样式类（新增 status 参数）
 */
export const getStatusDotClass = (statusCode?: unknown, status?: RequestStatus): string => {
  return dotClassByTone[getStatusTone(statusCode, status)]
}

/**
 * 获取容器背景色样式类（新增 status 参数）
 */
export const getStatusContainerClass = (statusCode?: unknown, status?: RequestStatus): string => {
  return containerClassByTone[getStatusTone(statusCode, status)]
}

/**
 * 格式化状态标签（新增 status 参数）
 */
export const formatStatusLabel = (statusCode?: unknown, status?: RequestStatus): string => {
  // 进行中时显示 "..."
  if (status === 'pending') return '...'

  if (typeof statusCode === 'number') return `${statusCode}`
  if (statusCode !== undefined && statusCode !== null) return String(statusCode)
  return '--'
}
