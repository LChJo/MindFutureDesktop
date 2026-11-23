/**
 * Apps composable for Coworks
 * Combines built-in apps with skill apps from installed plugins and local skills
 */

import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { BUILTIN_APPS } from '@/components/coworks/data/apps'
import type { BuiltinApp, SkillApp, CoworkApp } from '@/types/coworks-persistence'

// Local skill info type from backend
interface LocalSkillInfo {
  name: string
  title: string
  description: string
  path: string
}

// Plugin skill info type from backend
interface PluginSkillInfo {
  name: string
  title: string
  description: string
  plugin_name: string
  marketplace_name: string
  plugin_path: string
}

// Singleton state
const skillApps = ref<SkillApp[]>([])
const isLoading = ref(false)
const error = ref<string | null>(null)

export function useApps() {
  // Combine built-in apps and skill apps
  const allApps = computed<CoworkApp[]>(() => {
    return [...BUILTIN_APPS, ...skillApps.value]
  })

  // Get only built-in apps
  const builtinApps = computed<BuiltinApp[]>(() => {
    return BUILTIN_APPS
  })

  // Load skill apps from both local skills and installed plugins
  async function loadSkillApps(): Promise<void> {
    if (isLoading.value) return

    isLoading.value = true
    error.value = null

    try {
      // Load both local skills and plugin skills in parallel
      const [localSkills, pluginSkills] = await Promise.all([
        invoke<LocalSkillInfo[]>('get_local_skills'),
        invoke<PluginSkillInfo[]>('get_plugin_skills'),
      ])

      // Convert local skills to SkillApp format
      const localSkillApps: SkillApp[] = localSkills.map(skill => ({
        id: `skill-local-${skill.name}`,
        name: skill.title || skill.name,
        icon: 'fa-solid fa-wand-magic-sparkles',
        iconType: 'fontawesome' as const,
        iconCategory: 'local' as const,
        description: skill.description,
        mode: 'work' as const,
        prompt: skill.description,
        // Skill-specific fields
        isSkill: true as const,
        skillName: skill.name,
        pluginName: 'local',
        pluginPath: skill.path,
        marketplaceName: 'local',
      }))

      // Convert plugin skills to SkillApp format
      const pluginSkillApps: SkillApp[] = pluginSkills.map(skill => ({
        id: `skill-${skill.plugin_name}-${skill.name}`,
        name: skill.title || skill.name,
        icon: 'fa-solid fa-puzzle-piece',
        iconType: 'fontawesome' as const,
        iconCategory: 'plugin' as const,
        description: skill.description,
        mode: 'work' as const,
        prompt: skill.description,
        // Skill-specific fields
        isSkill: true as const,
        skillName: skill.name,
        pluginName: skill.plugin_name,
        pluginPath: skill.plugin_path,
        marketplaceName: skill.marketplace_name,
      }))

      // Combine local skills first, then plugin skills
      skillApps.value = [...localSkillApps, ...pluginSkillApps]
    } catch (e) {
      console.error('[useApps] Failed to load skill apps:', e)
      error.value = e instanceof Error ? e.message : 'Failed to load skill apps'
      skillApps.value = []
    } finally {
      isLoading.value = false
    }
  }

  // Find an app by ID
  function findAppById(id: string): CoworkApp | undefined {
    return allApps.value.find(app => app.id === id)
  }

  // Check if an app is a skill app
  function isSkillAppById(id: string): boolean {
    const app = findAppById(id)
    return app ? 'isSkill' in app && app.isSkill === true : false
  }

  // Get skill info from app ID (returns undefined if not a skill app)
  function getSkillInfo(appId: string): SkillApp | undefined {
    const app = findAppById(appId)
    if (app && 'isSkill' in app && app.isSkill) {
      return app as SkillApp
    }
    return undefined
  }

  return {
    allApps,
    builtinApps,
    skillApps,
    isLoading,
    error,
    loadSkillApps,
    findAppById,
    isSkillAppById,
    getSkillInfo,
  }
}
