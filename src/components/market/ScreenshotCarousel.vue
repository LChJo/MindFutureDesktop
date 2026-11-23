<template>
  <div
    v-if="screenshots.length > 0"
    class="screenshot-carousel"
  >
    <h4 class="carousel-title">
      截图预览
    </h4>

    <div class="carousel-container">
      <!-- Main Image -->
      <div
        class="main-image"
        @click="openLightbox"
      >
        <img
          :src="currentScreenshot.url"
          :alt="currentScreenshot.caption || '截图'"
        >
        <div
          v-if="currentScreenshot.caption"
          class="image-caption"
        >
          {{ currentScreenshot.caption }}
        </div>
      </div>

      <!-- Navigation -->
      <div
        v-if="screenshots.length > 1"
        class="carousel-nav"
      >
        <button
          class="nav-btn prev"
          :disabled="currentIndex === 0"
          @click="prev"
        >
          <i class="fa-solid fa-chevron-left" />
        </button>

        <div class="thumbnails">
          <div
            v-for="(screenshot, index) in screenshots"
            :key="index"
            class="thumbnail"
            :class="{ active: index === currentIndex }"
            @click="goTo(index)"
          >
            <img
              :src="screenshot.url"
              :alt="screenshot.caption || `截图 ${index + 1}`"
            >
          </div>
        </div>

        <button
          class="nav-btn next"
          :disabled="currentIndex === screenshots.length - 1"
          @click="next"
        >
          <i class="fa-solid fa-chevron-right" />
        </button>
      </div>

      <!-- Indicators -->
      <div
        v-if="screenshots.length > 1"
        class="indicators"
      >
        <span
          v-for="(_, index) in screenshots"
          :key="index"
          class="indicator"
          :class="{ active: index === currentIndex }"
          @click="goTo(index)"
        />
      </div>
    </div>

    <!-- Lightbox -->
    <Teleport to="body">
      <div
        v-if="showLightbox"
        class="lightbox"
        @click="closeLightbox"
      >
        <div
          class="lightbox-content"
          @click.stop
        >
          <button
            class="close-btn"
            @click="closeLightbox"
          >
            <i class="fa-solid fa-times" />
          </button>

          <img
            :src="currentScreenshot.url"
            :alt="currentScreenshot.caption || '截图'"
          >

          <div
            v-if="currentScreenshot.caption"
            class="lightbox-caption"
          >
            {{ currentScreenshot.caption }}
          </div>

          <div
            v-if="screenshots.length > 1"
            class="lightbox-nav"
          >
            <button
              class="nav-btn"
              :disabled="currentIndex === 0"
              @click.stop="prev"
            >
              <i class="fa-solid fa-chevron-left" />
            </button>
            <span class="counter">{{ currentIndex + 1 }} / {{ screenshots.length }}</span>
            <button
              class="nav-btn"
              :disabled="currentIndex === screenshots.length - 1"
              @click.stop="next"
            >
              <i class="fa-solid fa-chevron-right" />
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { PluginMetaScreenshot } from '@/types/pluginMarket'

const props = defineProps<{
  screenshots: PluginMetaScreenshot[]
}>()

const currentIndex = ref(0)
const showLightbox = ref(false)

const currentScreenshot = computed(() =>
  props.screenshots[currentIndex.value] || { url: '', caption: '' }
)

function prev() {
  if (currentIndex.value > 0) {
    currentIndex.value--
  }
}

function next() {
  if (currentIndex.value < props.screenshots.length - 1) {
    currentIndex.value++
  }
}

function goTo(index: number) {
  currentIndex.value = index
}

function openLightbox() {
  showLightbox.value = true
}

function closeLightbox() {
  showLightbox.value = false
}
</script>

<style lang="less" scoped>
.screenshot-carousel {
  margin-top: 20px;
}

.carousel-title {
  font-size: 14px;
  font-weight: 600;
  color: #333;
  margin: 0 0 12px 0;
}

.carousel-container {
  position: relative;
}

.main-image {
  position: relative;
  width: 100%;
  border-radius: 8px;
  overflow: hidden;
  cursor: zoom-in;
  background: #f5f5f5;

  img {
    width: 100%;
    height: auto;
    display: block;
    max-height: 300px;
    object-fit: contain;
  }

  .image-caption {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 8px 12px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.7));
    color: white;
    font-size: 12px;
  }
}

.carousel-nav {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.nav-btn {
  width: 32px;
  height: 32px;
  border: 1px solid #e0e0e0;
  border-radius: 50%;
  background: white;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;

  &:hover:not(:disabled) {
    border-color: #7c6eb8;
    color: #7c6eb8;
  }

  &:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  i {
    font-size: 12px;
  }
}

.thumbnails {
  display: flex;
  gap: 8px;
  flex: 1;
  overflow-x: auto;
  padding: 4px 0;

  .thumbnail {
    width: 60px;
    height: 40px;
    border-radius: 4px;
    overflow: hidden;
    cursor: pointer;
    border: 2px solid transparent;
    flex-shrink: 0;
    transition: border-color 0.2s;

    &:hover {
      border-color: #ccc;
    }

    &.active {
      border-color: #7c6eb8;
    }

    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
  }
}

.indicators {
  display: flex;
  justify-content: center;
  gap: 6px;
  margin-top: 12px;

  .indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ddd;
    cursor: pointer;
    transition: all 0.2s;

    &:hover {
      background: #bbb;
    }

    &.active {
      background: #7c6eb8;
      width: 20px;
      border-radius: 4px;
    }
  }
}

// Lightbox
.lightbox {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10100;
}

.lightbox-content {
  position: relative;
  max-width: 90vw;
  max-height: 90vh;

  img {
    max-width: 100%;
    max-height: 80vh;
    border-radius: 8px;
  }

  .close-btn {
    position: absolute;
    top: -40px;
    right: 0;
    width: 36px;
    height: 36px;
    border: none;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.2);
    color: white;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;

    &:hover {
      background: rgba(255, 255, 255, 0.3);
    }
  }

  .lightbox-caption {
    margin-top: 12px;
    color: white;
    text-align: center;
    font-size: 14px;
  }

  .lightbox-nav {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 20px;
    margin-top: 16px;

    .nav-btn {
      width: 40px;
      height: 40px;
      background: rgba(255, 255, 255, 0.2);
      border: none;
      color: white;

      &:hover:not(:disabled) {
        background: rgba(255, 255, 255, 0.3);
      }
    }

    .counter {
      color: white;
      font-size: 14px;
    }
  }
}
</style>
