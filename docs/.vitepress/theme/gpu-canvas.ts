// GPUI WebGPU / WebGL2 Live Canvas Renderer
import type { SerializedNode } from './playground-runtime';

export interface GpuContextResult {
  supported: boolean;
  rendererType: 'webgpu' | 'webgl2' | 'none';
  error?: string;
}

export interface RenderOptions {
  backdrop: 'acrylic' | 'mica' | 'catppuccin' | 'opaque';
}

interface HitResult {
  node: SerializedNode;
  onClickId?: number;
  cursorPointer?: boolean;
}

interface MeasuredSize {
  w: number;
  h: number;
}

export class GpuCanvasRenderer {
  private canvas: HTMLCanvasElement;
  private gl: WebGL2RenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private vao: WebGLVertexArrayObject | null = null;
  private vbo: WebGLBuffer | null = null;
  private textCanvas: HTMLCanvasElement;
  private textCtx: CanvasRenderingContext2D | null = null;
  private textTexture: WebGLTexture | null = null;
  private dpr = 1;
  private currentTree: SerializedNode | null = null;
  private measuredSizes = new WeakMap<SerializedNode, MeasuredSize>();
  private options: RenderOptions = {
    backdrop: 'catppuccin',
  };

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.textCanvas = document.createElement('canvas');
    this.textCtx = this.textCanvas.getContext('2d', { willReadFrequently: false });
  }

  static checkSupport(canvas: HTMLCanvasElement): GpuContextResult {
    const hasWebGPU = typeof navigator !== 'undefined' && 'gpu' in navigator;

    try {
      const gl = canvas.getContext('webgl2', {
        antialias: true,
        alpha: true,
        premultipliedAlpha: true,
      });

      if (gl) {
        return {
          supported: true,
          rendererType: hasWebGPU ? 'webgpu' : 'webgl2',
        };
      }
    } catch {
      // ignore
    }

    return {
      supported: false,
      rendererType: 'none',
      error: 'WebGPU or WebGL2 is required to view the live GPUI canvas. Please ensure hardware acceleration is enabled in your browser.',
    };
  }

  init(): boolean {
    const gl = this.canvas.getContext('webgl2', {
      antialias: true,
      alpha: true,
      premultipliedAlpha: true,
    });

    if (!gl) return false;
    this.gl = gl;

    const vsSource = `#version 300 es
      precision highp float;
      layout(location = 0) in vec2 a_pos;
      layout(location = 1) in vec2 a_uv;
      
      uniform vec2 u_resolution;
      
      out vec2 v_uv;
      out vec2 v_pixelPos;

      void main() {
        v_uv = a_uv;
        v_pixelPos = a_pos;
        vec2 zeroToOne = a_pos / u_resolution;
        vec2 zeroToTwo = zeroToOne * 2.0;
        vec2 clipSpace = zeroToTwo - 1.0;
        gl_Position = vec4(clipSpace * vec2(1.0, -1.0), 0.0, 1.0);
      }
    `;

    const fsSource = `#version 300 es
      precision highp float;
      in vec2 v_uv;
      in vec2 v_pixelPos;

      uniform vec4 u_rect; // x, y, width, height
      uniform vec4 u_color;
      uniform vec4 u_borderColor;
      uniform float u_borderWidth;
      uniform float u_borderRadius;
      uniform float u_opacity;
      uniform int u_isText;
      uniform sampler2D u_textTexture;

      out vec4 fragColor;

      float roundedBoxSDF(vec2 center, vec2 size, float radius) {
        vec2 q = abs(center) - size + radius;
        return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - radius;
      }

      void main() {
        if (u_isText == 1) {
          vec4 texColor = texture(u_textTexture, v_uv);
          fragColor = texColor * u_opacity;
          return;
        }

        vec2 rectCenter = u_rect.xy + u_rect.zw * 0.5;
        vec2 halfSize = u_rect.zw * 0.5;
        vec2 p = v_pixelPos - rectCenter;

        float dist = roundedBoxSDF(p, halfSize, u_borderRadius);

        // Anti-aliased outer edge
        float alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

        if (alpha <= 0.0) {
          discard;
        }

        vec4 col = u_color;

        // Render border if present
        if (u_borderWidth > 0.0 && u_borderColor.a > 0.0) {
          float borderDist = dist + u_borderWidth;
          float borderFactor = smoothstep(-0.5, 0.5, borderDist);
          col = mix(col, u_borderColor, borderFactor);
        }

        fragColor = vec4(col.rgb, col.a * alpha * u_opacity);
      }
    `;

    const program = this.createProgram(gl, vsSource, fsSource);
    if (!program) return false;
    this.program = program;

    this.vao = gl.createVertexArray();
    this.vbo = gl.createBuffer();

    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);

    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 4 * 4, 0);

    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 4 * 4, 2 * 4);

    gl.bindVertexArray(null);

    this.textTexture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, this.textTexture);
    // Nearest filtering gives sharpest pixel-perfect font rendering when 1:1 mapped
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

    return true;
  }

  resize(width: number, height: number): void {
    this.dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    const intW = Math.max(10, Math.floor(width));
    const intH = Math.max(10, Math.floor(height));

    this.canvas.style.width = `${intW}px`;
    this.canvas.style.height = `${intH}px`;
    this.canvas.width = Math.round(intW * this.dpr);
    this.canvas.height = Math.round(intH * this.dpr);

    if (this.gl) {
      this.gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    }
  }

  render(tree: SerializedNode, options?: Partial<RenderOptions>): void {
    if (!this.gl || !this.program) return;
    this.currentTree = tree;
    if (options) {
      this.options = { ...this.options, ...options };
    }

    const gl = this.gl;
    const width = this.canvas.width / this.dpr;
    const height = this.canvas.height / this.dpr;

    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    // Clear background
    const bgCol = this.getBackdropColor(this.options.backdrop);
    gl.clearColor(bgCol[0], bgCol[1], bgCol[2], bgCol[3]);
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.useProgram(this.program);

    const uRes = gl.getUniformLocation(this.program, 'u_resolution');
    gl.uniform2f(uRes, width, height);

    // Two-pass layout engine
    this.measureNode(tree, width);
    this.arrangeNode(tree, 0, 0, width, height);

    // Render tree nodes
    this.renderNode(tree);
  }

  hitTest(clientX: number, clientY: number): HitResult | null {
    if (!this.currentTree) return null;
    const rect = this.canvas.getBoundingClientRect();
    const x = clientX - rect.left;
    const y = clientY - rect.top;

    return this.hitTestRecursive(this.currentTree, x, y);
  }

  private hitTestRecursive(
    node: SerializedNode,
    x: number,
    y: number,
    inheritedClickId?: number,
    inheritedCursorPointer?: boolean
  ): HitResult | null {
    // Check if point is inside this node's bounds
    let isInside = false;
    if (node.layout) {
      const { x: nx, y: ny, w: nw, h: nh } = node.layout;
      isInside = x >= nx && x <= nx + nw && y >= ny && y <= ny + nh;
    }

    if (!isInside) {
      return null;
    }

    const currentClickId = node.onClickId ?? inheritedClickId;
    const currentCursorPointer = Boolean(node.cursorPointer || currentClickId !== undefined || inheritedCursorPointer);

    // Traverse children first (top-most in z-order)
    if (Array.isArray(node.children)) {
      for (let i = node.children.length - 1; i >= 0; i--) {
        const child = node.children[i];
        const hit = this.hitTestRecursive(child, x, y, currentClickId, currentCursorPointer);
        if (hit) return hit;
      }
    }

    // If this node or any enclosing ancestor has a click handler or cursor pointer flag
    if (currentClickId !== undefined || currentCursorPointer) {
      return {
        node,
        onClickId: currentClickId,
        cursorPointer: currentCursorPointer,
      };
    }

    return null;
  }

  // --- Two-Pass Flexbox Layout Engine ---

  private measureNode(node: SerializedNode, availW: number): MeasuredSize {
    const pad = node.p ?? 0;
    const px = node.px ?? pad;
    const py = node.py ?? pad;

    // 1. Text node measurement
    if (node.kind === 'text') {
      const text = node.text || '';
      const fontSize = node.fontSize || 14;
      const bold = Boolean(node.bold);

      let textWidth = 10;
      if (this.textCtx) {
        const fontStr = `${bold ? '600 ' : '400 '}${fontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`;
        this.textCtx.font = fontStr;
        const metrics = this.textCtx.measureText(text);
        textWidth = Math.ceil(metrics.width) + 4;
      } else {
        textWidth = Math.ceil(text.length * fontSize * 0.6) + 4;
      }

      const textHeight = Math.ceil(fontSize * 1.4);
      const measured = { w: textWidth, h: textHeight };
      this.measuredSizes.set(node, measured);
      return measured;
    }

    // 2. Button measurement
    if (node.kind === 'button') {
      const label = node.label || node.text || 'Button';
      let labelWidth = 50;
      if (this.textCtx) {
        this.textCtx.font = '600 14px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
        labelWidth = Math.ceil(this.textCtx.measureText(label).width);
      }
      const bpx = node.px ?? 16;
      const bpy = node.py ?? 8;
      const w = typeof node.w === 'number' ? node.w : Math.ceil(labelWidth + bpx * 2);
      const h = typeof node.h === 'number' ? node.h : Math.ceil(18 + bpy * 2);
      const measured = { w, h };
      this.measuredSizes.set(node, measured);
      return measured;
    }

    // 3. Badge measurement
    if (node.kind === 'badge') {
      const text = node.text || node.label || 'Badge';
      let textWidth = 30;
      if (this.textCtx) {
        this.textCtx.font = '600 12px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
        textWidth = Math.ceil(this.textCtx.measureText(text).width);
      }
      const bpx = node.px ?? 8;
      const bpy = node.py ?? 3;
      const w = typeof node.w === 'number' ? node.w : Math.ceil(textWidth + bpx * 2);
      const h = typeof node.h === 'number' ? node.h : Math.ceil(16 + bpy * 2);
      const measured = { w, h };
      this.measuredSizes.set(node, measured);
      return measured;
    }

    // 4. Input measurement
    if (node.kind === 'input') {
      const w = typeof node.w === 'number' ? node.w : availW;
      const h = typeof node.h === 'number' ? node.h : 38;
      const measured = { w, h };
      this.measuredSizes.set(node, measured);
      return measured;
    }

    // 5. Divider measurement
    if (node.kind === 'divider') {
      const w = node.hFull ? (typeof node.w === 'number' ? node.w : 1) : availW;
      const h = node.hFull ? 36 : (typeof node.h === 'number' ? node.h : 1);
      const measured = { w, h };
      this.measuredSizes.set(node, measured);
      return measured;
    }

    // 6. Containers (div, card, row, col, stack)
    const innerAvailW = Math.max(0, (typeof node.w === 'number' ? node.w : availW) - px * 2);
    const isRow = Boolean(node.flexRow || node.kind === 'row');
    const gap = node.gap ?? (node.kind === 'card' ? 12 : 8);

    let naturalW = 0;
    let naturalH = 0;

    if (Array.isArray(node.children) && node.children.length > 0) {
      if (isRow) {
        let totalChildW = 0;
        let maxChildH = 0;

        for (const child of node.children) {
          const childSize = this.measureNode(child, innerAvailW);
          totalChildW += childSize.w;
          maxChildH = Math.max(maxChildH, childSize.h);
        }

        const totalGaps = Math.max(0, node.children.length - 1) * gap;
        naturalW = totalChildW + totalGaps + px * 2;
        naturalH = maxChildH + py * 2;
      } else {
        // Column
        let maxChildW = 0;
        let totalChildH = 0;

        for (const child of node.children) {
          const childSize = this.measureNode(child, innerAvailW);
          maxChildW = Math.max(maxChildW, childSize.w);
          totalChildH += childSize.h;
        }

        const totalGaps = Math.max(0, node.children.length - 1) * gap;
        naturalW = maxChildW + px * 2;
        naturalH = totalChildH + totalGaps + py * 2;
      }
    } else {
      naturalW = px * 2;
      naturalH = py * 2;
    }

    let finalW = naturalW;
    if (typeof node.w === 'number') {
      finalW = node.w;
    } else if (node.wFull) {
      finalW = availW;
    }

    let finalH = naturalH;
    if (typeof node.h === 'number') {
      finalH = node.h;
    }

    const measured = { w: finalW, h: finalH };
    this.measuredSizes.set(node, measured);
    return measured;
  }

  private arrangeNode(node: SerializedNode, x: number, y: number, width: number, height: number): void {
    const pad = node.p ?? 0;
    const px = node.px ?? pad;
    const py = node.py ?? pad;

    let finalW = width;
    let finalH = height;

    if (typeof node.w === 'number') {
      finalW = node.w;
    } else if (!node.wFull) {
      const measured = this.measuredSizes.get(node);
      if (measured) finalW = Math.min(width, measured.w);
    }

    if (typeof node.h === 'number') {
      finalH = node.h;
    } else if (!node.hFull) {
      const measured = this.measuredSizes.get(node);
      if (measured) finalH = Math.min(height, measured.h);
    }

    node.layout = { x, y, w: finalW, h: finalH };

    if (!Array.isArray(node.children) || node.children.length === 0) return;

    const innerX = x + px;
    const innerY = y + py;
    const innerW = Math.max(0, finalW - px * 2);
    const innerH = Math.max(0, finalH - py * 2);

    const isRow = Boolean(node.flexRow || node.kind === 'row');
    const gap = node.gap ?? (node.kind === 'card' ? 12 : 8);

    if (isRow) {
      // Row arrangement
      let totalChildrenW = 0;
      for (const ch of node.children) {
        const m = this.measuredSizes.get(ch) || { w: 40, h: 30 };
        totalChildrenW += m.w;
      }
      totalChildrenW += Math.max(0, node.children.length - 1) * gap;

      let startX = innerX;
      let actualGap = gap;

      if ((node.justifyBetween || node.justify_between) && node.children.length > 1) {
        let childSum = 0;
        for (const ch of node.children) {
          childSum += (this.measuredSizes.get(ch)?.w || 40);
        }
        actualGap = Math.max(0, (innerW - childSum) / (node.children.length - 1));
        startX = innerX;
      } else if (node.justifyCenter || node.justify_center || node.justify === 'center') {
        startX = innerX + Math.max(0, (innerW - totalChildrenW) / 2);
      } else if (node.justifyEnd || node.justify_end || node.justify === 'end') {
        startX = innerX + Math.max(0, innerW - totalChildrenW);
      }
      let curX = startX;
      for (const ch of node.children) {
        const m = this.measuredSizes.get(ch) || { w: 40, h: 30 };
        const chW = ch.wFull ? innerW : m.w;
        const chH = ch.hFull ? innerH : m.h;

        let chY = innerY;
        if (node.itemsCenter || node.items_center || node.items === 'center') {
          chY = innerY + Math.max(0, (innerH - chH) / 2);
        } else if (node.itemsEnd || node.items_end || node.items === 'end') {
          chY = innerY + Math.max(0, innerH - chH);
        }

        this.arrangeNode(ch, curX, chY, chW, chH);
        curX += chW + actualGap;
      }
    } else {
      // Column arrangement
      let totalChildrenH = 0;
      for (const ch of node.children) {
        const m = this.measuredSizes.get(ch) || { w: innerW, h: 30 };
        totalChildrenH += m.h;
      }
      totalChildrenH += Math.max(0, node.children.length - 1) * gap;

      let startY = innerY;
      let actualGap = gap;

      if ((node.justifyBetween || node.justify_between) && node.children.length > 1) {
        let childSum = 0;
        for (const ch of node.children) {
          childSum += (this.measuredSizes.get(ch)?.h || 30);
        }
        actualGap = Math.max(0, (innerH - childSum) / (node.children.length - 1));
        startY = innerY;
      } else if (node.justifyCenter || node.justify_center || node.justify === 'center') {
        startY = innerY + Math.max(0, (innerH - totalChildrenH) / 2);
      } else if (node.justifyEnd || node.justify_end || node.justify === 'end') {
        startY = innerY + Math.max(0, innerH - totalChildrenH);
      }
      let curY = startY;
      for (const ch of node.children) {
        const m = this.measuredSizes.get(ch) || { w: innerW, h: 30 };
        const chW = ch.wFull ? innerW : (typeof ch.w === 'number' ? ch.w : m.w);
        const chH = ch.hFull ? innerH : (typeof ch.h === 'number' ? ch.h : m.h);

        let chX = innerX;
        if (node.itemsCenter || node.items_center || node.items === 'center') {
          chX = innerX + Math.max(0, (innerW - chW) / 2);
        } else if (node.itemsEnd || node.items_end || node.items === 'end') {
          chX = innerX + Math.max(0, innerW - chW);
        }

        this.arrangeNode(ch, chX, curY, chW, chH);
        curY += chH + actualGap;
      }
    }
  }

  private renderNode(node: SerializedNode): void {
    if (!this.gl || !this.program || !node.layout) return;

    const { x, y, w, h } = node.layout;
    if (w <= 0 || h <= 0) return;

    // 1. Draw Background Quad & Borders
    const bgCol = this.getNodeBgColor(node);
    const borderCol = this.parseColor(node.borderColor || '#313244', 1.0);
    const borderWidth = node.border ?? (node.kind === 'card' ? 1 : 0);
    const borderRadius = node.rounded ?? (node.kind === 'badge' ? 999 : (node.kind === 'card' ? 8 : (node.kind === 'button' ? 6 : 0)));
    const opacity = node.opacity ?? (node.disabled ? 0.5 : 1.0);

    if (bgCol[3] > 0 || borderWidth > 0) {
      this.drawQuad(x, y, w, h, bgCol, borderCol, borderWidth, borderRadius, opacity);
    }

    // 2. Render Text Content (strictly for text, button, badge, input)
    // Containers NEVER render their own text to prevent duplicated overlapping text!
    if (node.kind === 'text') {
      const text = node.text || '';
      if (text) {
        const textColor = this.parseColor(node.textColor || node.color || '#cdd6f4', 1.0);
        const fontSize = node.fontSize || 14;
        const bold = Boolean(node.bold);
        this.drawText(text, x, y + h / 2, fontSize, textColor, bold, 'left');
      }
    } else if (node.kind === 'button') {
      const label = node.label || node.text || '';
      if (label) {
        const textColor = this.parseColor(node.textColor || '#ffffff', 1.0);
        this.drawText(label, x + w / 2, y + h / 2, 14, textColor, true, 'center');
      }
    } else if (node.kind === 'badge') {
      const text = node.text || node.label || '';
      if (text) {
        const textColor = this.parseColor(node.textColor || node.color || '#89b4fa', 1.0);
        this.drawText(text, x + w / 2, y + h / 2, 12, textColor, true, 'center');
      }
    } else if (node.kind === 'input') {
      const text = node.text || node.value || node.placeholder || '';
      if (text) {
        const textColor = this.parseColor(node.textColor || '#cdd6f4', 1.0);
        const bpx = node.px ?? 12;
        this.drawText(text, x + bpx, y + h / 2, 14, textColor, false, 'left');
      }
    }

    // 3. Render Children Recursively
    if (Array.isArray(node.children)) {
      for (const ch of node.children) {
        this.renderNode(ch);
      }
    }
  }

  private drawQuad(
    x: number,
    y: number,
    w: number,
    h: number,
    color: [number, number, number, number],
    borderColor: [number, number, number, number],
    borderWidth: number,
    borderRadius: number,
    opacity: number
  ): void {
    if (!this.gl || !this.program || !this.vao || !this.vbo) return;
    const gl = this.gl;

    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);

    // Snap to physical pixel boundary
    const snapX = Math.round(x * this.dpr) / this.dpr;
    const snapY = Math.round(y * this.dpr) / this.dpr;
    const snapW = Math.round(w * this.dpr) / this.dpr;
    const snapH = Math.round(h * this.dpr) / this.dpr;

    const x1 = snapX;
    const y1 = snapY;
    const x2 = snapX + snapW;
    const y2 = snapY + snapH;

    const vertices = new Float32Array([
      x1, y1, 0, 0,
      x2, y1, 1, 0,
      x1, y2, 0, 1,
      x1, y2, 0, 1,
      x2, y1, 1, 0,
      x2, y2, 1, 1,
    ]);

    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.DYNAMIC_DRAW);

    const uRect = gl.getUniformLocation(this.program, 'u_rect');
    const uColor = gl.getUniformLocation(this.program, 'u_color');
    const uBorderColor = gl.getUniformLocation(this.program, 'u_borderColor');
    const uBorderWidth = gl.getUniformLocation(this.program, 'u_borderWidth');
    const uBorderRadius = gl.getUniformLocation(this.program, 'u_borderRadius');
    const uOpacity = gl.getUniformLocation(this.program, 'u_opacity');
    const uIsText = gl.getUniformLocation(this.program, 'u_isText');

    gl.uniform4f(uRect, snapX, snapY, snapW, snapH);
    gl.uniform4fv(uColor, color);
    gl.uniform4fv(uBorderColor, borderColor);
    gl.uniform1f(uBorderWidth, borderWidth);
    gl.uniform1f(uBorderRadius, borderRadius);
    gl.uniform1f(uOpacity, opacity);
    gl.uniform1i(uIsText, 0);

    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.bindVertexArray(null);
  }

  private drawText(
    text: string,
    x: number,
    y: number,
    size: number,
    color: [number, number, number, number],
    bold: boolean,
    align: 'left' | 'center' = 'left',
    fontFamily?: string
  ): void {
    if (!this.gl || !this.program || !this.textCtx || !this.textTexture || !this.vao || !this.vbo || !text) return;
    const gl = this.gl;
    const ctx = this.textCtx;
    const dpr = this.dpr;

    // Rasterize at high-DPI resolution
    const fontStr = `${bold ? '600 ' : '400 '}${Math.round(size * dpr)}px ${fontFamily ? `"${fontFamily}", ` : ''}-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`;

    // 1. Set font on context before measuring
    ctx.font = fontStr;
    const metrics = ctx.measureText(text);
    const padX = Math.ceil(2 * dpr);
    const physicalW = Math.max(1, Math.ceil(metrics.width) + padX * 2);
    const physicalH = Math.max(1, Math.ceil(size * 1.45 * dpr));

    const logicalW = physicalW / dpr;
    const logicalH = physicalH / dpr;

    this.textCanvas.width = physicalW;
    this.textCanvas.height = physicalH;

    // 2. Re-apply font and styles after canvas dimension reset
    ctx.clearRect(0, 0, physicalW, physicalH);
    ctx.font = fontStr;
    ctx.fillStyle = `rgba(${Math.round(color[0] * 255)}, ${Math.round(color[1] * 255)}, ${Math.round(color[2] * 255)}, ${color[3]})`;
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'left';
    ctx.fillText(text, padX, physicalH / 2);

    gl.bindTexture(gl.TEXTURE_2D, this.textTexture);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.textCanvas);

    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);

    let renderX = x - (padX / dpr);
    if (align === 'center') {
      renderX = x - logicalW / 2;
    }
    const renderY = y - logicalH / 2;
    // Strict physical pixel alignment guarantees 1:1 mapping with ZERO blur
    const snapX = Math.round(renderX * dpr) / dpr;
    const snapY = Math.round(renderY * dpr) / dpr;

    const vertices = new Float32Array([
      snapX, snapY, 0, 0,
      snapX + logicalW, snapY, 1, 0,
      snapX, snapY + logicalH, 0, 1,
      snapX, snapY + logicalH, 0, 1,
      snapX + logicalW, snapY, 1, 0,
      snapX + logicalW, snapY + logicalH, 1, 1,
    ]);

    gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.DYNAMIC_DRAW);

    const uRect = gl.getUniformLocation(this.program, 'u_rect');
    const uColor = gl.getUniformLocation(this.program, 'u_color');
    const uBorderColor = gl.getUniformLocation(this.program, 'u_borderColor');
    const uBorderWidth = gl.getUniformLocation(this.program, 'u_borderWidth');
    const uBorderRadius = gl.getUniformLocation(this.program, 'u_borderRadius');
    const uOpacity = gl.getUniformLocation(this.program, 'u_opacity');
    const uIsText = gl.getUniformLocation(this.program, 'u_isText');

    gl.uniform4f(uRect, snapX, snapY, logicalW, logicalH);
    gl.uniform4f(uColor, 1, 1, 1, 1);
    gl.uniform4f(uBorderColor, 0, 0, 0, 0);
    gl.uniform1f(uBorderWidth, 0);
    gl.uniform1f(uBorderRadius, 0);
    gl.uniform1f(uOpacity, 1.0);
    gl.uniform1i(uIsText, 1);

    gl.drawArrays(gl.TRIANGLES, 0, 6);
    gl.bindVertexArray(null);
  }


  private getNodeBgColor(node: SerializedNode): [number, number, number, number] {
    if (node.bg) {
      return this.parseColor(node.bg, 1.0);
    }
    if (node.kind === 'card') {
      return [0.118, 0.118, 0.18, 1.0]; // Catppuccin Base #1e1e2e
    }
    if (node.kind === 'button') {
      if (node.variant === 'secondary') return [0.192, 0.196, 0.267, 1.0];
      if (node.variant === 'ghost') return [0, 0, 0, 0];
      if (node.variant === 'danger') return [0.937, 0.267, 0.267, 1.0];
      return [0.231, 0.51, 0.965, 1.0]; // Primary blue
    }
    if (node.kind === 'badge') {
      return [0.192, 0.196, 0.267, 1.0];
    }
    if (node.kind === 'input') {
      return [0.094, 0.094, 0.145, 1.0];
    }
    return [0, 0, 0, 0];
  }

  private getBackdropColor(backdrop: RenderOptions['backdrop']): [number, number, number, number] {
    switch (backdrop) {
      case 'acrylic':
        return [0.09, 0.1, 0.15, 0.88];
      case 'mica':
        return [0.12, 0.12, 0.17, 0.94];
      case 'opaque':
        return [0.067, 0.067, 0.106, 1.0];
      case 'catppuccin':
      default:
        return [0.118, 0.118, 0.18, 1.0];
    }
  }

  private parseColor(colorStr: string, defaultAlpha = 1.0): [number, number, number, number] {
    if (colorStr.startsWith('#')) {
      const hex = colorStr.slice(1);
      if (hex.length === 6) {
        const r = parseInt(hex.substring(0, 2), 16) / 255;
        const g = parseInt(hex.substring(2, 4), 16) / 255;
        const b = parseInt(hex.substring(4, 6), 16) / 255;
        return [r, g, b, defaultAlpha];
      }
      if (hex.length === 8) {
        const r = parseInt(hex.substring(0, 2), 16) / 255;
        const g = parseInt(hex.substring(2, 4), 16) / 255;
        const b = parseInt(hex.substring(4, 6), 16) / 255;
        const a = parseInt(hex.substring(6, 8), 16) / 255;
        return [r, g, b, a];
      }
      if (hex.length === 3) {
        const r = parseInt(hex[0] + hex[0], 16) / 255;
        const g = parseInt(hex[1] + hex[1], 16) / 255;
        const b = parseInt(hex[2] + hex[2], 16) / 255;
        return [r, g, b, defaultAlpha];
      }
    }

    if (colorStr === 'transparent') {
      return [0, 0, 0, 0];
    }

    return [0.8, 0.84, 0.96, defaultAlpha];
  }

  private createProgram(gl: WebGL2RenderingContext, vsSource: string, fsSource: string): WebGLProgram | null {
    const vs = gl.createShader(gl.VERTEX_SHADER);
    const fs = gl.createShader(gl.FRAGMENT_SHADER);
    if (!vs || !fs) return null;

    gl.shaderSource(vs, vsSource);
    gl.compileShader(vs);
    if (!gl.getShaderParameter(vs, gl.COMPILE_STATUS)) {
      console.error('VS Error:', gl.getShaderInfoLog(vs));
      return null;
    }

    gl.shaderSource(fs, fsSource);
    gl.compileShader(fs);
    if (!gl.getShaderParameter(fs, gl.COMPILE_STATUS)) {
      console.error('FS Error:', gl.getShaderInfoLog(fs));
      return null;
    }

    const program = gl.createProgram();
    if (!program) return null;

    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Program Error:', gl.getProgramInfoLog(program));
      return null;
    }

    return program;
  }
}
