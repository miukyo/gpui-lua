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
import counterExample from '../../../../crates/gpui_lua/examples/counter.lua?raw';
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
  background-color: #09090b;
  color: #fafafa;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid #27272a;
  margin: 1rem 0;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
}

/* Topbar */
.playground-topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background-color: #121214;
  padding: 0.6rem 1rem;
  border-bottom: 1px solid #27272a;
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
  color: #a1a1aa;
  font-weight: 600;
}

.backdrop-select {
  background-color: #18181b;
  color: #fafafa;
  border: 1px solid #27272a;
  border-radius: 6px;
  padding: 0.35rem 0.6rem;
  font-size: 0.85rem;
  cursor: pointer;
  outline: none;
}

.backdrop-select:focus {
  border-color: #ffffff;
}

.preset-desc {
  font-size: 0.8rem;
  color: #71717a;
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
  color: #a1a1aa;
  cursor: pointer;
  user-select: none;
}

.toggle-label input[type='checkbox'] {
  accent-color: #ffffff;
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
  background-color: rgba(255, 255, 255, 0.08);
  color: #ffffff;
  border: 1px solid #3f3f46;
}
.status-ready .status-dot {
  background-color: #ffffff;
  box-shadow: 0 0 6px #ffffff;
}
.status-evaluating,
.status-init {
  background-color: rgba(255, 255, 255, 0.05);
  color: #a1a1aa;
  border: 1px solid #27272a;
}
.status-evaluating .status-dot,
.status-init .status-dot {
  background-color: #a1a1aa;
}

.status-error {
  background-color: rgba(255, 255, 255, 0.05);
  color: #e4e4e7;
  border: 1px solid #52525b;
}
.status-error .status-dot {
  background-color: #e4e4e7;
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
  background-color: #ffffff;
  color: #000000;
}
.btn-primary:hover {
  background-color: #e4e4e7;
}
.btn-secondary {
  background-color: #18181b;
  color: #fafafa;
  border: 1px solid #27272a;
}
.btn-secondary:hover {
  background-color: #27272a;
}
.btn-text {
  background: transparent;
  border: none;
  color: #fafafa;
  font-size: 0.75rem;
  cursor: pointer;
  padding: 0.2rem 0.4rem;
  border-radius: 4px;
}
.btn-text:hover {
  background-color: rgba(255, 255, 255, 0.08);
}

/* Panes */
.workbench-panes {
  display: flex;
  flex: 1;
  min-height: 0;
  border-bottom: 1px solid #27272a;
}

.pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.pane-editor {
  border-right: 1px solid #27272a;
  background-color: #09090b;
}
.editor-header,
.preview-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.45rem 0.8rem;
  background-color: #121214;
  border-bottom: 1px solid #27272a;
  font-size: 0.8rem;
  color: #a1a1aa;
}

.editor-title,
.toolbar-title {
  font-weight: 600;
  color: #fafafa;
}
.editor-hint {
  font-size: 0.75rem;
  color: #71717a;
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
  background-color: #000000;
}

.toolbar-group {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.toolbar-label {
  font-size: 0.8rem;
  color: #a1a1aa;
}

.viewport-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  background: radial-gradient(circle at 50% 50%, #121214 0%, #000000 100%);
  overflow: hidden;
  position: relative;
}

.canvas-container {
  width: 100%;
  height: 100%;
  border-radius: 8px;
  overflow: hidden;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  border: 1px solid #27272a;
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
  background-color: #121214;
  border: 1px solid #3f3f46;
  border-radius: 8px;
  padding: 1.2rem;
  color: #fafafa;
}

.callout-icon {
  font-size: 1.6rem;
}

.callout-content h3 {
  margin: 0 0 0.4rem 0;
  font-size: 1rem;
  color: #ffffff;
}

.callout-content p {
  margin: 0;
  font-size: 0.85rem;
  color: #a1a1aa;
  line-height: 1.4;
}

/* Log Drawer */
.log-drawer {
  display: flex;
  flex-direction: column;
  background-color: #09090b;
  border-top: 1px solid #27272a;
  transition: max-height 0.2s ease;
  max-height: 200px;
  min-height: 34px;
  flex-shrink: 0;
}

.log-drawer.is-collapsed {
  max-height: 34px;
}

.drawer-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.4rem 0.8rem;
  background-color: #121214;
  border-bottom: 1px solid #27272a;
  cursor: pointer;
  user-select: none;
}

.drawer-title {
  font-size: 0.8rem;
  font-weight: 600;
  color: #a1a1aa;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.log-count {
  color: #71717a;
  font-weight: normal;
}

.drawer-actions {
  display: flex;
  gap: 0.5rem;
}

.drawer-body {
  height: 160px;
  overflow-y: auto;
  padding: 0.5rem 0.8rem;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  font-size: 12px;
  background-color: #09090b;
}

.log-empty {
  color: #71717a;
  font-style: italic;
  padding: 0.4rem 0;
}

.log-row {
  display: flex;
  align-items: baseline;
  gap: 0.6rem;
  padding: 0.25rem 0;
  line-height: 1.4;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.log-time {
  color: #71717a;
  font-size: 11px;
  flex-shrink: 0;
}

.log-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 3px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  flex-shrink: 0;
}

.log-info .log-badge {
  background-color: #18181b;
  color: #ffffff;
  border: 1px solid #3f3f46;
}

.log-reactive .log-badge {
  background-color: #27272a;
  color: #e4e4e7;
  border: 1px solid #3f3f46;
}

.log-warn .log-badge {
  background-color: #27272a;
  color: #d4d4d8;
  border: 1px solid #3f3f46;
}

.log-error .log-badge {
  background-color: #27272a;
  color: #ffffff;
  border: 1px solid #52525b;
}

.log-msg {
  color: #fafafa;
  word-break: break-all;
  white-space: pre-wrap;
}
@media (max-width: 900px) {
  .workbench-panes {
    flex-direction: column;
  }
  .pane-editor {
    border-right: none;
    border-bottom: 1px solid #27272a;
    height: 50%;
  }
}
</style>
