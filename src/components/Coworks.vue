<script setup lang="ts">
import CoworksApp from './coworks/CoworksApp.vue'

defineProps<{
  windowId?: string
  windowData?: Record<string, unknown>
}>()
</script>

<template>
  <CoworksApp />
</template>

<style scoped>
/* CoworksApp handles its own styling */
</style>
