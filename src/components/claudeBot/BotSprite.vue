<script setup lang="ts">
import { computed } from 'vue'
import type { BotDefinition } from '@/types/claude-bot'

const props = defineProps<{
  bot: BotDefinition
  size?: number
  channelDisconnected?: boolean
}>()

const pixelSize = computed(() => Math.floor((props.size ?? 64) / 16))
const svgSize = computed(() => pixelSize.value * 16)

const animClass = computed(() => {
  switch (props.bot.status) {
    case 'working': return 'anim-working'
    case 'error': return 'anim-error'
    case 'stopped': return 'anim-stopped'
    case 'paused': return 'anim-paused'
    default: return 'anim-idle'
  }
})

const containerStyle = computed(() => {
  if (props.channelDisconnected) {
    return { filter: 'drop-shadow(0 0 6px rgba(230, 85, 13, 0.5))' }
  }
  return {}
})

const spritePixels = computed(() => {
  const c = props.bot.color
  const v = props.bot.spriteVariant % 6
  return getSpriteData(v, c)
})

function getSpriteData(variant: number, accent: string): [number, number, string][] {
  const body = '#8b949e'
  const dark = '#6e7681'
  const eye = '#e6edf3'
  const eyeDot = accent

  const sprites: [number, number, string][][] = [
    // Variant 0: Classic sitting frog — front view, big protruding eyes
    [
      // Eyes (bulging above head)
      [5,1,eye],[6,1,eye],[9,1,eye],[10,1,eye],
      [5,2,eye],[6,2,eyeDot],[9,2,eyeDot],[10,2,eye],
      // Head
      [5,3,body],[6,3,body],[7,3,body],[8,3,body],[9,3,body],[10,3,body],
      [4,4,body],[5,4,body],[6,4,body],[7,4,body],[8,4,body],[9,4,body],[10,4,body],[11,4,body],
      // Mouth
      [4,5,body],[5,5,body],[6,5,accent],[7,5,accent],[8,5,accent],[9,5,accent],[10,5,body],[11,5,body],
      // Body with belly
      [4,6,body],[5,6,body],[6,6,dark],[7,6,dark],[8,6,dark],[9,6,dark],[10,6,body],[11,6,body],
      [5,7,body],[6,7,dark],[7,7,dark],[8,7,dark],[9,7,dark],[10,7,body],
      [3,8,body],[5,8,body],[6,8,dark],[7,8,dark],[8,8,dark],[9,8,dark],[10,8,body],[12,8,body],
      [2,9,body],[3,9,body],[4,9,body],[5,9,body],[6,9,body],[7,9,body],[8,9,body],[9,9,body],[10,9,body],[11,9,body],[12,9,body],[13,9,body],
      // Front feet
      [2,10,accent],[3,10,accent],[12,10,accent],[13,10,accent],
      // Back legs
      [4,10,body],[5,10,body],[10,10,body],[11,10,body],
      [3,11,body],[4,11,body],[11,11,body],[12,11,body],
      // Back feet
      [2,12,accent],[3,12,accent],[4,12,accent],[11,12,accent],[12,12,accent],[13,12,accent],
    ],
    // Variant 1: Round toad — extra wide and pudgy, wart spots
    [
      // Eyes
      [6,2,eye],[7,2,eye],[9,2,eye],[10,2,eye],
      [6,3,eye],[7,3,eyeDot],[9,3,eyeDot],[10,3,eye],
      // Wide head
      [4,4,body],[5,4,body],[6,4,body],[7,4,body],[8,4,body],[9,4,body],[10,4,body],[11,4,body],
      [3,5,body],[4,5,body],[5,5,body],[6,5,body],[7,5,body],[8,5,body],[9,5,body],[10,5,body],[11,5,body],[12,5,body],
      // Wart spots
      [3,6,body],[4,6,body],[5,6,accent],[6,6,body],[7,6,body],[8,6,body],[9,6,body],[10,6,accent],[11,6,body],[12,6,body],
      // Belly
      [3,7,body],[4,7,body],[5,7,dark],[6,7,dark],[7,7,dark],[8,7,dark],[9,7,dark],[10,7,dark],[11,7,body],[12,7,body],
      [3,8,body],[4,8,body],[5,8,dark],[6,8,dark],[7,8,dark],[8,8,dark],[9,8,dark],[10,8,dark],[11,8,body],[12,8,body],
      [3,9,body],[4,9,body],[5,9,body],[6,9,body],[7,9,body],[8,9,body],[9,9,body],[10,9,body],[11,9,body],[12,9,body],
      [4,10,body],[5,10,body],[6,10,body],[7,10,body],[8,10,body],[9,10,body],[10,10,body],[11,10,body],
      // Stubby legs and feet
      [3,11,accent],[4,11,accent],[5,11,body],[10,11,body],[11,11,accent],[12,11,accent],
      [2,12,accent],[3,12,accent],[12,12,accent],[13,12,accent],
    ],
    // Variant 2: Jumping frog — legs extended, dynamic pose
    [
      // Eyes
      [5,1,eye],[6,1,eye],[9,1,eye],[10,1,eye],
      [5,2,eye],[6,2,eyeDot],[9,2,eyeDot],[10,2,eye],
      // Head
      [5,3,body],[6,3,body],[7,3,body],[8,3,body],[9,3,body],[10,3,body],
      [4,4,body],[5,4,body],[6,4,body],[7,4,body],[8,4,body],[9,4,body],[10,4,body],[11,4,body],
      // Belly
      [4,5,body],[5,5,dark],[6,5,dark],[7,5,dark],[8,5,dark],[9,5,dark],[10,5,dark],[11,5,body],
      // Front arms reaching forward
      [2,6,accent],[3,6,body],[4,6,body],[5,6,dark],[6,6,dark],[7,6,dark],[8,6,dark],[9,6,dark],[10,6,dark],[11,6,body],[12,6,body],[13,6,accent],
      [2,7,accent],[3,7,body],[4,7,body],[5,7,body],[6,7,body],[7,7,body],[8,7,body],[9,7,body],[10,7,body],[11,7,body],[12,7,body],[13,7,accent],
      // Narrow waist
      [6,8,body],[7,8,body],[8,8,body],[9,8,body],
      // Back legs extending down
      [5,9,body],[6,9,body],[9,9,body],[10,9,body],
      [4,10,body],[5,10,body],[10,10,body],[11,10,body],
      [4,11,body],[11,11,body],
      [3,12,accent],[4,12,accent],[11,12,accent],[12,12,accent],
      [2,13,accent],[3,13,accent],[12,13,accent],[13,13,accent],
    ],
    // Variant 3: Wide flat toad — extra wide and squat
    [
      // Eyes (spread wide)
      [4,3,eye],[5,3,eye],[10,3,eye],[11,3,eye],
      [4,4,eye],[5,4,eyeDot],[10,4,eyeDot],[11,4,eye],
      // Very wide body
      [2,5,body],[3,5,body],[4,5,body],[5,5,body],[6,5,body],[7,5,body],[8,5,body],[9,5,body],[10,5,body],[11,5,body],[12,5,body],[13,5,body],
      [2,6,body],[3,6,body],[4,6,accent],[5,6,body],[6,6,body],[7,6,body],[8,6,body],[9,6,body],[10,6,body],[11,6,accent],[12,6,body],[13,6,body],
      // Wide belly
      [2,7,body],[3,7,body],[4,7,dark],[5,7,dark],[6,7,dark],[7,7,dark],[8,7,dark],[9,7,dark],[10,7,dark],[11,7,dark],[12,7,body],[13,7,body],
      [2,8,body],[3,8,body],[4,8,dark],[5,8,dark],[6,8,dark],[7,8,dark],[8,8,dark],[9,8,dark],[10,8,dark],[11,8,dark],[12,8,body],[13,8,body],
      [2,9,body],[3,9,body],[4,9,body],[5,9,body],[6,9,body],[7,9,body],[8,9,body],[9,9,body],[10,9,body],[11,9,body],[12,9,body],[13,9,body],
      // Short legs and feet
      [1,10,body],[2,10,accent],[3,10,body],[4,10,body],[11,10,body],[12,10,body],[13,10,accent],[14,10,body],
      [1,11,accent],[2,11,accent],[13,11,accent],[14,11,accent],
    ],
    // Variant 4: Tree frog — slender body, large toe pads
    [
      // Large eyes
      [6,1,eye],[7,1,eye],[8,1,eye],[9,1,eye],
      [5,2,eye],[6,2,eye],[7,2,eyeDot],[8,2,eyeDot],[9,2,eye],[10,2,eye],
      // Narrow head
      [6,3,body],[7,3,body],[8,3,body],[9,3,body],
      [5,4,body],[6,4,body],[7,4,body],[8,4,body],[9,4,body],[10,4,body],
      [5,5,body],[6,5,body],[7,5,accent],[8,5,accent],[9,5,body],[10,5,body],
      // Slim body
      [5,6,body],[6,6,dark],[7,6,dark],[8,6,dark],[9,6,dark],[10,6,body],
      [5,7,body],[6,7,dark],[7,7,dark],[8,7,dark],[9,7,dark],[10,7,body],
      // Long arms with toe pads
      [2,8,accent],[3,8,body],[5,8,body],[6,8,body],[7,8,body],[8,8,body],[9,8,body],[10,8,body],[12,8,body],[13,8,accent],
      [2,9,accent],[3,9,body],[6,9,body],[7,9,body],[8,9,body],[9,9,body],[12,9,body],[13,9,accent],
      // Waist
      [7,10,body],[8,10,body],
      // Long back legs
      [6,11,body],[7,11,body],[8,11,body],[9,11,body],
      [5,12,body],[6,12,body],[9,12,body],[10,12,body],
      // Large toe pads
      [4,13,accent],[5,13,accent],[10,13,accent],[11,13,accent],
      [3,14,accent],[4,14,accent],[11,14,accent],[12,14,accent],
    ],
    // Variant 5: Big-eyed frog — enormous eyes, small body
    [
      // Oversized eyes (3x3 each)
      [4,0,eye],[5,0,eye],[6,0,eye],[9,0,eye],[10,0,eye],[11,0,eye],
      [4,1,eye],[5,1,eye],[6,1,eye],[9,1,eye],[10,1,eye],[11,1,eye],
      [4,2,eye],[5,2,eyeDot],[6,2,eye],[9,2,eye],[10,2,eyeDot],[11,2,eye],
      // Small head
      [5,3,body],[6,3,body],[7,3,body],[8,3,body],[9,3,body],[10,3,body],
      [5,4,body],[6,4,body],[7,4,body],[8,4,body],[9,4,body],[10,4,body],
      // Mouth
      [4,5,body],[5,5,body],[6,5,accent],[7,5,accent],[8,5,accent],[9,5,accent],[10,5,body],[11,5,body],
      // Tiny body with belly
      [4,6,body],[5,6,body],[6,6,dark],[7,6,dark],[8,6,dark],[9,6,dark],[10,6,body],[11,6,body],
      [4,7,body],[5,7,body],[6,7,dark],[7,7,dark],[8,7,dark],[9,7,dark],[10,7,body],[11,7,body],
      [5,8,body],[6,8,body],[7,8,body],[8,8,body],[9,8,body],[10,8,body],
      // Small legs
      [3,9,accent],[4,9,body],[5,9,body],[10,9,body],[11,9,body],[12,9,accent],
      [3,10,accent],[12,10,accent],
      // Back legs and feet
      [5,11,body],[6,11,body],[9,11,body],[10,11,body],
      [4,12,accent],[5,12,accent],[10,12,accent],[11,12,accent],
    ],
  ]

  return sprites[variant] ?? sprites[0]
}
</script>

<template>
  <div
    class="bot-sprite-container"
    :class="animClass"
    :style="containerStyle"
  >
    <svg
      :width="svgSize"
      :height="svgSize"
      :viewBox="`0 0 ${svgSize} ${svgSize}`"
      class="bot-svg"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect
        v-for="(pixel, idx) in spritePixels"
        :key="idx"
        :x="pixel[0] * pixelSize"
        :y="pixel[1] * pixelSize"
        :width="pixelSize"
        :height="pixelSize"
        :fill="pixel[2]"
      />
    </svg>
    <div
      v-if="channelDisconnected"
      class="network-error-badge"
      title="频道连接已断开"
    >
      <i class="fa-solid fa-wifi-slash" />
    </div>
    <div class="bot-name-label">
      {{ bot.name }}
    </div>
  </div>
</template>

<style scoped lang="less">
.bot-sprite-container {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  transition: transform 0.3s ease;

  &:hover {
    transform: scale(1.1);
  }
}

.bot-svg {
  image-rendering: pixelated;
  image-rendering: crisp-edges;
}

.bot-name-label {
  font-size: 10px;
  color: #e6edf3;
  text-align: center;
  max-width: 80px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

// Network disconnected badge
.network-error-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #e6550d;
  color: #fff;
  font-size: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  animation: badge-pulse 1.5s ease-in-out infinite;
  z-index: 10;
}

@keyframes badge-pulse {
  0%, 100% {
    opacity: 1;
    box-shadow: 0 0 0 0 rgba(230, 85, 13, 0.5);
  }
  50% {
    opacity: 0.85;
    box-shadow: 0 0 0 5px rgba(230, 85, 13, 0);
  }
}

// Idle: gentle bounce
.anim-idle {
  animation: bot-idle 2s ease-in-out infinite;
}

@keyframes bot-idle {
  0%, 100% { transform: translateY(0) scaleY(1); }
  20% { transform: translateY(0) scaleY(0.9) scaleX(1.05); }
  40% { transform: translateY(-6px) scaleY(1.05); }
  60% { transform: translateY(0) scaleY(0.95); }
  80% { transform: translateY(0) scaleY(1); }
}

// Working: glow pulse
.anim-working {
  animation: bot-working 1s ease-in-out infinite;
}

@keyframes bot-working {
  0%, 100% { filter: brightness(1) drop-shadow(0 0 4px rgba(63, 185, 80, 0.3)); }
  50% { filter: brightness(1.3) drop-shadow(0 0 8px rgba(63, 185, 80, 0.6)); }
}

// Error: red flash
.anim-error {
  animation: bot-error 0.8s ease-in-out infinite;
}

@keyframes bot-error {
  0%, 100% { opacity: 1; filter: none; }
  50% { opacity: 0.6; filter: hue-rotate(-60deg) saturate(2); }
}

// Stopped: greyed out
.anim-stopped {
  filter: grayscale(0.8);
  opacity: 0.5;
  animation: none;
}

// Paused: slow pulse
.anim-paused {
  animation: bot-paused 3s ease-in-out infinite;
}

@keyframes bot-paused {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.6; }
}
</style>
