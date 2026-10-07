<template>
  <div class="playground-root">
    <!-- Top Workbench Header -->
    <header class="playground-topbar">
      <div class="topbar-left">
        <span class="topbar-label">Example: {{ example.name }}</span>
        <span class="preset-desc">{{ example.description }}</span>
      </div>

      <div class="topbar-right">
        <!-- VM Status Pill -->
        <span :class="['status-pill', `status-${vmState}`]">
          <span class="status-dot"></span>
          {{ vmStatusText }}
        </span>

        <label class="toggle-label" title="Automatically re-run when Lua code changes">
          <input type="checkbox" v-model="liveReRender" />
          <span>Live Re-render</span>
        </label>

        <button class="btn btn-secondary" @click="copyLuaCode" title="Copy code to clipboard">
          {{ copied ? 'Copied!' : 'Copy Lua' }}
        </button>
        <button class="btn btn-secondary" @click="resetCode" title="Reset code to preset default">
          Reset
        </button>
        <button class="btn btn-primary" @click="runCode" title="Execute script">
          ▶ Run
        </button>
      </div>
    </header>

    <!-- Main Workbench Area (2 Panes) -->
    <div class="workbench-panes">
      <!-- Left Pane: CodeMirror Editor -->
      <section class="pane pane-editor">
        <div class="editor-header">
          <span class="editor-title">main.lua (GPUI-CE DSL)</span>
          <span class="editor-hint">Ctrl+Enter to Run</span>
        </div>
        <div ref="editorContainer" class="editor-container"></div>
      </section>

      <!-- Right Pane: Canvas Viewport & Controls -->
      <section class="pane pane-preview">
        <!-- Viewport Header Toolbar -->
        <div class="preview-toolbar">
          <div class="toolbar-group">
            <span class="toolbar-title">GPUI Canvas Viewport</span>
          </div>

          <div class="toolbar-group">
            <!-- Backdrop Switcher -->
            <label for="backdrop-select" class="toolbar-label">Backdrop:</label>
            <select
              id="backdrop-select"
              v-model="selectedBackdrop"
              class="backdrop-select"
              @change="reRenderCanvas"
            >
              <option value="catppuccin">Catppuccin Base</option>
              <option value="acrylic">Acrylic Blur</option>
              <option value="mica">Mica</option>
              <option value="opaque">Opaque</option>
            </select>
          </div>
        </div>

        <!-- Viewport Container -->
        <div class="viewport-wrapper">
          <!-- Unsupported Hardware/Context Warning Card -->
          <div v-if="gpuError" class="unsupported-callout">
            <div class="callout-icon">⚠️</div>
            <div class="callout-content">
              <h3>Hardware Acceleration Required</h3>
              <p>{{ gpuError }}</p>
            </div>
          </div>

          <!-- Live GPUI Canvas (No DOM/CSS fallback) -->
          <div v-show="!gpuError" class="canvas-container" ref="canvasContainer">
            <canvas
              ref="canvasEl"
              id="gpui-web-canvas"
              class="gpui-web-canvas"
              @click="handleCanvasClick"
              @mousemove="handleCanvasMouseMove"
            ></canvas>
          </div>
        </div>
      </section>
    </div>

    <!-- Bottom Drawer: Output & Logs -->
    <div :class="['log-drawer', { 'is-collapsed': isDrawerCollapsed }]">
      <div class="drawer-header" @click="isDrawerCollapsed = !isDrawerCollapsed">
        <div class="drawer-title">
          <span>Console & Reactive State Transitions</span>
          <span class="log-count">({{ logs.length }})</span>
        </div>
        <div class="drawer-actions" @click.stop>
          <button class="btn-text" @click="clearLogs">Clear</button>
          <button class="btn-text" @click="isDrawerCollapsed = !isDrawerCollapsed">
            {{ isDrawerCollapsed ? '▲ Expand' : '▼ Collapse' }}
          </button>
        </div>
      </div>

      <div v-show="!isDrawerCollapsed" class="drawer-body" ref="logBody">
        <div v-if="logs.length === 0" class="log-empty">
          No logs or events emitted yet. Execute code or interact with the canvas to observe reactive updates.
        </div>
        <div
          v-for="log in logs"
          :key="log.id"
          :class="['log-row', `log-${log.type}`]"
        >
          <span class="log-time">[{{ log.time }}]</span>
          <span class="log-badge">{{ log.type.toUpperCase() }}</span>
          <span class="log-msg">{{ log.message }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from 'vue';
import { EditorView, basicSetup } from 'codemirror';
import { StreamLanguage } from '@codemirror/language';
import { lua } from '@codemirror/legacy-modes/mode/lua';
import { oneDark } from '@codemirror/theme-one-dark';
import counterExample from '../../../../examples/counter.lua?raw';
import { PlaygroundRuntime } from '../playground-runtime';
import type { LogEntry, SerializedNode } from '../playground-runtime';
import { GpuCanvasRenderer } from '../gpu-canvas';

// Workbench state
const example = {
  name: 'Reactive Counter',
  description: 'signal() state with increment / decrement / reset buttons',
  code: counterExample,
};

const vmState = ref<'init' | 'ready' | 'evaluating' | 'error'>('init');
const liveReRender = ref<boolean>(true);
const copied = ref<boolean>(false);
const selectedBackdrop = ref<'acrylic' | 'mica' | 'catppuccin' | 'opaque'>('catppuccin');
const gpuError = ref<string | null>(null);
const logs = ref<LogEntry[]>([]);
const isDrawerCollapsed = ref<boolean>(false);
let isSwitchingPreset = false;

// DOM refs
const editorContainer = ref<HTMLDivElement | null>(null);
const canvasContainer = ref<HTMLDivElement | null>(null);
const canvasEl = ref<HTMLCanvasElement | null>(null);
const logBody = ref<HTMLDivElement | null>(null);

// Engine instances
let editorView: EditorView | null = null;
let runtime: PlaygroundRuntime | null = null;
let renderer: GpuCanvasRenderer | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let resizeObserver: ResizeObserver | null = null;

const vmStatusText = computed<string>(() => {
  switch (vmState.value) {
    case 'init':
      return 'Loading Wasm...';
    case 'ready':
      return 'Ready (Luau Pure-Rust)';
    case 'evaluating':
      return 'Evaluating...';
    case 'error':
      return 'Error';
    default:
      return 'Unknown';
  }
});

onMounted(async () => {
  initCodeMirror();
  initGpuRenderer();
  await initRuntime();
  await runCode();

  // Watch container size changes to resize GPU canvas
  if (canvasContainer.value) {
    resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (renderer && width > 0 && height > 0) {
          renderer.resize(width, height);
          reRenderCanvas();
        }
      }
    });
    resizeObserver.observe(canvasContainer.value);
  }
});

onBeforeUnmount(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
  if (editorView) {
    editorView.destroy();
  }
});

function initCodeMirror(): void {
  if (!editorContainer.value) return;

  const initialCode = example?.code || '';

  editorView = new EditorView({
    doc: initialCode,
    extensions: [
      basicSetup,
      StreamLanguage.define(lua),
      oneDark,
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (update.docChanged && liveReRender.value && !isSwitchingPreset) {
          if (debounceTimer) clearTimeout(debounceTimer);
          debounceTimer = setTimeout(() => {
            runCode();
          }, 300);
        }
      }),
      EditorView.domEventHandlers({
        keydown: (event) => {
          if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
            event.preventDefault();
            runCode();
            return true;
          }
          return false;
        },
      }),
    ],
    parent: editorContainer.value,
  });
}

function initGpuRenderer(): void {
  if (!canvasEl.value) return;

  const check = GpuCanvasRenderer.checkSupport(canvasEl.value);
  if (!check.supported) {
    gpuError.value = check.error || 'WebGPU or WebGL2 context failed to initialize.';
    return;
  }

  renderer = new GpuCanvasRenderer(canvasEl.value);
  const success = renderer.init();
  if (!success) {
    gpuError.value = 'Failed to compile GPUI Web GPU shaders or create vertex pipeline.';
    return;
  }

  if (canvasContainer.value) {
    const rect = canvasContainer.value.getBoundingClientRect();
    renderer.resize(rect.width || 600, rect.height || 450);
  }
}

async function initRuntime(): Promise<void> {
  vmState.value = 'init';
  runtime = new PlaygroundRuntime(
    // Reactive signal listener
    (key, val) => {
      if (key === '__click__' && val && typeof val === 'object') {
        lastParsedTree = val as SerializedNode;
      }
      if (renderer) {
        reRenderCanvas();
      }
    },
    // Print listener
    (entry) => {
      logs.value.push(entry);
      scrollLogsToBottom();
    }
  );

  try {
    await runtime.init();
    vmState.value = 'ready';
  } catch (err: unknown) {
    vmState.value = 'error';
    const msg = err instanceof Error ? err.message : String(err);
    logs.value.push({
      id: Date.now(),
      type: 'error',
      message: `Failed to initialize Luau VM: ${msg}`,
      time: new Date().toLocaleTimeString(),
    });
  }
}
let lastParsedTree: SerializedNode | null = null;

async function runCode(): Promise<void> {
  if (!runtime || !editorView) return;

  const code = editorView.state.doc.toString();
  vmState.value = 'evaluating';

  try {
    const tree = await runtime.runLuaCode(code);
    lastParsedTree = tree;
    vmState.value = 'ready';

    if (renderer) {
      renderer.render(tree, {
        backdrop: selectedBackdrop.value,
      });
    }
  } catch {
    vmState.value = 'error';
  }
}

function reRenderCanvas(): void {
  if (renderer && lastParsedTree) {
    renderer.render(lastParsedTree, {
      backdrop: selectedBackdrop.value,
    });
  }
}

async function handleCanvasClick(event: MouseEvent): Promise<void> {
  if (!renderer || !runtime) return;

  const hit = renderer.hitTest(event.clientX, event.clientY);
  if (hit && hit.onClickId !== undefined) {
    try {
      const updatedTree = await runtime.triggerClick(hit.onClickId);
      if (updatedTree) {
        lastParsedTree = updatedTree;
        reRenderCanvas();
      }
    } catch {
      // logged by runtime
    }
  }
}

function handleCanvasMouseMove(event: MouseEvent): void {
  if (!renderer || !canvasEl.value) return;

  const hit = renderer.hitTest(event.clientX, event.clientY);
  if (hit && (hit.onClickId !== undefined || hit.cursorPointer)) {
    canvasEl.value.style.cursor = 'pointer';
  } else {
    canvasEl.value.style.cursor = 'default';
  }
}

function resetCode(): void {
  if (!editorView || !example) return;

  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }

  if (runtime) {
    runtime.resetStore();
  }

  isSwitchingPreset = true;
  editorView.dispatch({
    changes: {
      from: 0,
      to: editorView.state.doc.length,
      insert: example.code,
    },
  });
  isSwitchingPreset = false;

  runCode();
}

async function copyLuaCode(): Promise<void> {
  if (!editorView) return;
  const code = editorView.state.doc.toString();
  try {
    await navigator.clipboard.writeText(code);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    // ignore
  }
}

function clearLogs(): void {
  logs.value = [];
}

function scrollLogsToBottom(): void {
  setTimeout(() => {
    if (logBody.value) {
      logBody.value.scrollTop = logBody.value.scrollHeight;
    }
  }, 10);
}
</script>

<style scoped>
.playground-root {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 120px);
  min-height: 700px;
  background-color: #11111b;
  color: #cdd6f4;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid #313244;
  margin: 1rem 0;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
}

/* Topbar */
.playground-topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background-color: #181825;
  padding: 0.6rem 1rem;
  border-bottom: 1px solid #313244;
  flex-wrap: wrap;
  gap: 0.8rem;
}

.topbar-left,
.topbar-right {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.topbar-label {
  font-size: 0.85rem;
  color: #a6adc8;
  font-weight: 600;
}

.backdrop-select {
  background-color: #1e1e2e;
  color: #cdd6f4;
  border: 1px solid #45475a;
  border-radius: 6px;
  padding: 0.35rem 0.6rem;
  font-size: 0.85rem;
  cursor: pointer;
  outline: none;
}

.backdrop-select:focus {
  border-color: #89b4fa;
}

.preset-desc {
  font-size: 0.8rem;
  color: #6c7086;
  max-width: 320px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.toggle-label {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.8rem;
  color: #a6adc8;
  cursor: pointer;
  user-select: none;
}

.toggle-label input[type='checkbox'] {
  accent-color: #89b4fa;
}

/* Status Pill */
.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.25rem 0.65rem;
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
}

.status-ready {
  background-color: rgba(166, 227, 161, 0.15);
  color: #a6e3a1;
  border: 1px solid rgba(166, 227, 161, 0.3);
}
.status-ready .status-dot {
  background-color: #a6e3a1;
  box-shadow: 0 0 6px #a6e3a1;
}

.status-evaluating,
.status-init {
  background-color: rgba(249, 226, 175, 0.15);
  color: #f9e2af;
  border: 1px solid rgba(249, 226, 175, 0.3);
}
.status-evaluating .status-dot,
.status-init .status-dot {
  background-color: #f9e2af;
  animation: pulse 1s infinite alternate;
}

.status-error {
  background-color: rgba(243, 139, 168, 0.15);
  color: #f38ba8;
  border: 1px solid rgba(243, 139, 168, 0.3);
}
.status-error .status-dot {
  background-color: #f38ba8;
}

@keyframes pulse {
  from { opacity: 0.4; }
  to { opacity: 1; }
}

/* Buttons */
.btn {
  padding: 0.35rem 0.8rem;
  font-size: 0.85rem;
  font-weight: 600;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  border: none;
}

.btn-primary {
  background-color: #89b4fa;
  color: #11111b;
}
.btn-primary:hover {
  background-color: #b4befe;
}

.btn-secondary {
  background-color: #313244;
  color: #cdd6f4;
  border: 1px solid #45475a;
}
.btn-secondary:hover {
  background-color: #45475a;
}

.btn-text {
  background: transparent;
  border: none;
  color: #89b4fa;
  font-size: 0.75rem;
  cursor: pointer;
  padding: 0.2rem 0.4rem;
  border-radius: 4px;
}
.btn-text:hover {
  background-color: rgba(137, 180, 250, 0.1);
}

/* Panes */
.workbench-panes {
  display: flex;
  flex: 1;
  min-height: 0;
  border-bottom: 1px solid #313244;
}

.pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.pane-editor {
  border-right: 1px solid #313244;
  background-color: #181825;
}

.editor-header,
.preview-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.45rem 0.8rem;
  background-color: #1e1e2e;
  border-bottom: 1px solid #313244;
  font-size: 0.8rem;
  color: #a6adc8;
}

.editor-title,
.toolbar-title {
  font-weight: 600;
  color: #cdd6f4;
}

.editor-hint {
  font-size: 0.75rem;
  color: #6c7086;
}

.editor-container {
  flex: 1;
  overflow: auto;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 13px;
}

:deep(.cm-editor) {
  height: 100%;
}
:deep(.cm-scroller) {
  overflow: auto;
}

/* Preview Pane */
.pane-preview {
  background-color: #11111b;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.toolbar-label {
  font-size: 0.8rem;
  color: #a6adc8;
}

.viewport-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  background: radial-gradient(circle at 50% 50%, #1e1e2e 0%, #11111b 100%);
  overflow: hidden;
  position: relative;
}

.canvas-container {
  width: 100%;
  height: 100%;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  border: 1px solid #313244;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.gpui-web-canvas {
  display: block;
  touch-action: none;
  outline: none;
  image-rendering: -webkit-optimize-contrast;
  image-rendering: pixelated;
}

/* Hardware Error Callout */
.unsupported-callout {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  max-width: 480px;
  background-color: rgba(243, 139, 168, 0.1);
  border: 1px solid #f38ba8;
  border-radius: 8px;
  padding: 1.2rem;
  color: #cdd6f4;
}

.callout-icon {
  font-size: 1.6rem;
}

.callout-content h3 {
  margin: 0 0 0.4rem 0;
  font-size: 1rem;
  color: #f38ba8;
}

.callout-content p {
  margin: 0;
  font-size: 0.85rem;
  color: #a6adc8;
  line-height: 1.4;
}

/* Log Drawer */
.log-drawer {
  display: flex;
  flex-direction: column;
  background-color: #181825;
  border-top: 1px solid #313244;
  transition: max-height 0.2s ease;
  max-height: 180px;
  height: 100%;
}

.log-drawer.is-collapsed {
  max-height: 34px;
}

.drawer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.35rem 0.8rem;
  background-color: #1e1e2e;
  border-bottom: 1px solid #313244;
  cursor: pointer;
  user-select: none;
}

.drawer-title {
  font-size: 0.78rem;
  font-weight: 600;
  color: #a6adc8;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.log-count {
  color: #6c7086;
  font-weight: normal;
}

.drawer-actions {
  display: flex;
  gap: 0.5rem;
}

.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: 0.4rem 0.8rem;
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  font-size: 12px;
}

.log-empty {
  color: #6c7086;
  font-style: italic;
  padding: 0.4rem 0;
}

.log-row {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  padding: 0.15rem 0;
  line-height: 1.4;
}

.log-time {
  color: #6c7086;
  font-size: 11px;
}

.log-badge {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 4px;
  border-radius: 3px;
}

.log-info .log-badge {
  background-color: rgba(137, 180, 250, 0.2);
  color: #89b4fa;
}

.log-reactive .log-badge {
  background-color: rgba(166, 227, 161, 0.2);
  color: #a6e3a1;
}

.log-error .log-badge {
  background-color: rgba(243, 139, 168, 0.2);
  color: #f38ba8;
}

.log-msg {
  color: #cdd6f4;
  word-break: break-all;
}

@media (max-width: 900px) {
  .workbench-panes {
    flex-direction: column;
  }
  .pane-editor {
    border-right: none;
    border-bottom: 1px solid #313244;
    height: 50%;
  }
}
</style>
