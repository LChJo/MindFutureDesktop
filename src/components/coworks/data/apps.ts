import type { BuiltinApp } from '@/types/coworks-persistence'

export const BUILTIN_APPS: BuiltinApp[] = [
  {
    id: 'file-organizer',
    name: '文件整理',
    icon: '📁',
    description: '智能整理和归类文件',
    mode: 'work',
    prompt: '帮助用户整理和归类目录中的文件，包括重命名、移动、分类等操作。',
  },
  {
    id: 'code-reviewer',
    name: '代码审查',
    icon: '🔍',
    description: '审查和改进代码质量',
    mode: 'work',
    prompt: '审查代码质量，发现潜在问题，提供改进建议和最佳实践。',
  },
  {
    id: 'data-analyzer',
    name: '数据分析',
    icon: '📊',
    description: '分析和可视化数据',
    mode: 'work',
    prompt: '分析数据文件，提取关键信息，生成数据报告。',
  },
  {
    id: 'translator',
    name: '翻译助手',
    icon: '🌐',
    description: '多语言翻译和本地化',
    mode: 'work',
    prompt: '翻译文档和代码注释，支持多种语言之间的转换。',
  },
]
