<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const greetMsg = ref("");
const name = ref("");

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsg.value = await invoke("greet", { name: name.value });
}
</script>

<template>
  <main class="container">
    <header class="header">
      <h1>🧠 MindFuture Desktop</h1>
      <p class="subtitle">Claude 图形化窗口管理界面 | Claude Graphical Window Management Interface</p>
    </header>

    <div class="content">
      <div class="welcome-section">
        <h2>欢迎使用 MindFuture Desktop</h2>
        <p>基于 Tauri v2 和 Vue 3 构建的现代化桌面应用</p>
        
        <div class="features">
          <div class="feature-card">
            <div class="feature-icon">🚀</div>
            <h3>轻量高效</h3>
            <p>基于 Tauri v2，体积小巧，性能卓越</p>
          </div>
          
          <div class="feature-card">
            <div class="feature-icon">🎨</div>
            <h3>现代化 UI</h3>
            <p>使用 Vue 3 + TypeScript 开发</p>
          </div>
          
          <div class="feature-card">
            <div class="feature-icon">🪟</div>
            <h3>窗口管理</h3>
            <p>图形化界面，操作简便</p>
          </div>
        </div>
      </div>

      <div class="demo-section">
        <h3>快速测试</h3>
        <form class="greet-form" @submit.prevent="greet">
          <input 
            id="greet-input" 
            v-model="name" 
            placeholder="输入您的名字 / Enter your name..." 
          />
          <button type="submit">打招呼 / Greet</button>
        </form>
        <p v-if="greetMsg" class="greet-result">{{ greetMsg }}</p>
      </div>

      <div class="tech-stack">
        <p>技术栈 | Tech Stack</p>
        <div class="logos">
          <a href="https://tauri.app" target="_blank">
            <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
          </a>
          <a href="https://vuejs.org/" target="_blank">
            <img src="./assets/vue.svg" class="logo vue" alt="Vue logo" />
          </a>
          <a href="https://vite.dev" target="_blank">
            <img src="/vite.svg" class="logo vite" alt="Vite logo" />
          </a>
        </div>
      </div>
    </div>
  </main>
</template>

<style scoped>
.header {
  text-align: center;
  padding: 2rem 0;
  border-bottom: 2px solid #e0e0e0;
  margin-bottom: 2rem;
}

.header h1 {
  font-size: 2.5rem;
  margin: 0;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.subtitle {
  margin-top: 0.5rem;
  color: #666;
  font-size: 0.95rem;
}

.content {
  max-width: 1000px;
  margin: 0 auto;
  padding: 0 2rem;
}

.welcome-section {
  text-align: center;
  margin-bottom: 3rem;
}

.welcome-section h2 {
  font-size: 1.8rem;
  margin-bottom: 0.5rem;
}

.features {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1.5rem;
  margin-top: 2rem;
}

.feature-card {
  background: #ffffff;
  border-radius: 12px;
  padding: 1.5rem;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  transition: transform 0.3s ease, box-shadow 0.3s ease;
}

.feature-card:hover {
  transform: translateY(-5px);
  box-shadow: 0 8px 12px rgba(0, 0, 0, 0.15);
}

.feature-icon {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.feature-card h3 {
  margin: 0.5rem 0;
  font-size: 1.2rem;
}

.feature-card p {
  margin: 0;
  color: #666;
  font-size: 0.9rem;
}

.demo-section {
  background: #f8f9fa;
  border-radius: 12px;
  padding: 2rem;
  margin-bottom: 2rem;
  text-align: center;
}

.demo-section h3 {
  margin-top: 0;
}

.greet-form {
  display: flex;
  justify-content: center;
  gap: 0.5rem;
  margin-top: 1rem;
}

.greet-result {
  margin-top: 1rem;
  padding: 1rem;
  background: #e8f5e9;
  border-radius: 8px;
  color: #2e7d32;
  font-weight: 500;
}

.tech-stack {
  text-align: center;
  padding: 2rem 0;
}

.tech-stack p {
  margin-bottom: 1rem;
  color: #666;
}

.logos {
  display: flex;
  justify-content: center;
  gap: 2rem;
}

.logo {
  height: 4em;
  padding: 1em;
  will-change: filter;
  transition: filter 0.3s;
}

.logo:hover {
  filter: drop-shadow(0 0 2em rgba(102, 126, 234, 0.5));
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

@media (prefers-color-scheme: dark) {
  .header {
    border-bottom-color: #444;
  }

  .subtitle {
    color: #aaa;
  }

  .feature-card {
    background: #1e1e1e;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  }

  .feature-card:hover {
    box-shadow: 0 8px 12px rgba(0, 0, 0, 0.4);
  }

  .feature-card p {
    color: #aaa;
  }

  .demo-section {
    background: #1e1e1e;
  }

  .greet-result {
    background: #1b5e20;
    color: #a5d6a7;
  }

  .tech-stack p {
    color: #aaa;
  }
}
</style>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif, "PingFang SC", "Microsoft YaHei";
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding: 0;
  min-height: 100vh;
}

body {
  margin: 0;
  padding: 0;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #ffffff;
}

button:hover {
  opacity: 0.9;
}

button:active {
  opacity: 0.8;
}

input,
button {
  outline: none;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }

  button {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: #ffffff;
  }
}
</style>