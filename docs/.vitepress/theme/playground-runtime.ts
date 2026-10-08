import init, { run } from './ulua/ulua_web.js';

export interface LogEntry {
  id: number;
  type: 'info' | 'warn' | 'error';
  message: string;
  time: string;
}

export interface SerializedNode {
  kind: 'div' | 'text' | 'button' | 'card' | 'badge' | 'row' | 'stack' | 'col' | 'input' | 'divider';
  text?: string;
  label?: string;
  value?: string;
  placeholder?: string;
  bg?: string;
  textColor?: string;
  color?: string;
  borderColor?: string;
  border?: number;
  rounded?: number;
  p?: number;
  px?: number;
  py?: number;
  m?: number;
  mx?: number;
  my?: number;
  w?: number;
  h?: number;
  wFull?: boolean;
  hFull?: boolean;
  gap?: number;
  flexRow?: boolean;
  flexCol?: boolean;
  itemsCenter?: boolean;
  items_center?: boolean;
  itemsEnd?: boolean;
  items_end?: boolean;
  justifyCenter?: boolean;
  justify_center?: boolean;
  justifyBetween?: boolean;
  justify_between?: boolean;
  justifyEnd?: boolean;
  justify_end?: boolean;
  fontSize?: number;
  fontFamily?: string;
  bold?: boolean;
  variant?: string;
  disabled?: boolean;
  opacity?: number;
  onClickId?: number;
  cursorPointer?: boolean;
  children?: SerializedNode[];
  layout?: { x: number; y: number; w: number; h: number };
}

export class PlaygroundRuntime {
  private isInitialized = false;
  private onSignal?: (key: string, val: any) => void;
  private onPrint?: (entry: LogEntry) => void;
  private signals: Record<string, any> = {};
  private lastUserCode: string = '';
  private lastRuntimeError: string | null = null;

  constructor(
    onSignal?: (key: string, val: any) => void,
    onPrint?: (entry: LogEntry) => void
  ) {
    this.onSignal = onSignal;
    this.onPrint = onPrint;

    if (typeof globalThis !== 'undefined') {
      (globalThis as any).__uluaOnRuntimeError = (msg: string) => {
        this.lastRuntimeError = msg;
      };
    }
  }

  public emitLog(type: 'info' | 'warn' | 'error', message: string): void {
    if (this.onPrint) {
      this.onPrint({
        id: Date.now() + Math.random(),
        type,
        message,
        time: new Date().toLocaleTimeString(),
      });
    }
  }

  public async init(): Promise<void> {
    if (this.isInitialized) return;

    const path = typeof window !== 'undefined' ? window.location.pathname : '';
    const base = path.startsWith('/gpui-lua/')
      ? '/gpui-lua/'
      : path.startsWith('/gpui.lua/')
      ? '/gpui.lua/'
      : '/';
    await init({ module_or_path: `${base}ulua_web_bg.wasm` });
    this.isInitialized = true;
  }

  public resetStore(): void {
    this.signals = {};
  }

  private serializeSignalsToLua(): string {
    const parts: string[] = [];
    for (const [k, v] of Object.entries(this.signals)) {
      const valStr = typeof v === 'string' ? JSON.stringify(v) : (typeof v === 'boolean' || typeof v === 'number' ? String(v) : 'nil');
      parts.push(`[${JSON.stringify(k)}] = ${valStr}`);
    }
    return '{ ' + parts.join(', ') + ' }';
  }

  private buildHarness(userCode: string, triggerClickId?: number): string {
    const luaSignals = this.serializeSignalsToLua();
    const triggerIdStr = triggerClickId !== undefined ? String(triggerClickId) : 'nil';

    return `
      local function to_json_val(v, is_array)
        if type(v) == 'string' then
          return string.format('%q', v)
        elseif type(v) == 'number' or type(v) == 'boolean' then
          return tostring(v)
        elseif type(v) == 'table' then
          if is_array or #v > 0 or (next(v) == nil and is_array) then
            local parts = {}
            for _, x in ipairs(v) do
              table.insert(parts, to_json_val(x, false))
            end
            return '[' .. table.concat(parts, ',') .. ']'
          else
            local parts = {}
            for k, x in pairs(v) do
              if type(k) == 'string' then
                local is_ch = (k == 'children')
                table.insert(parts, string.format('%q:%s', k, to_json_val(x, is_ch)))
              end
            end
            return '{' .. table.concat(parts, ',') .. '}'
          end
        end
        return 'null'
      end

      __stored_signals = ${luaSignals}
      __click_callbacks = {}
      __next_click_id = 1
      __trigger_click = ${triggerIdStr}

      function signal(initial_value, name)
        local key = name or tostring({})
        if __stored_signals[key] == nil then
          __stored_signals[key] = initial_value
        end
        local function get()
          return __stored_signals[key]
        end
        local function set(new_val)
          __stored_signals[key] = new_val
          return new_val
        end
        return get, set
      end

      colors = {
        hex = function(s) return s end,
        rgb = function(r, g, b) return string.format('#%02x%02x%02x', r, g, b) end,
        rgba = function(r, g, b, a) return string.format('#%02x%02x%02x%02x', r, g, b, math.floor(a * 255)) end
      }

      local ElementBuilder = {}
      ElementBuilder.__index = ElementBuilder

      function ElementBuilder.new(kind, props)
        local self = setmetatable({}, ElementBuilder)
        self.kind = kind or 'div'
        self.props = props or {}
        self.children = {}
        self.text_content = nil
        self.click_id = nil

        if self.props.children then
          for _, c in ipairs(self.props.children) do
            table.insert(self.children, c)
          end
        end

        if self.props.on_click then
          local id = __next_click_id
          __next_click_id = __next_click_id + 1
          __click_callbacks[id] = self.props.on_click
          self.click_id = id
        end

        return self
      end

      function ElementBuilder:child(ch)
        if ch then table.insert(self.children, ch) end
        return self
      end

      function ElementBuilder:w(v) self.props.w = v; return self end
      function ElementBuilder:h(v) self.props.h = v; return self end
      function ElementBuilder:w_full() self.props.w_full = true; return self end
      function ElementBuilder:h_full() self.props.h_full = true; return self end
      function ElementBuilder:bg(c) self.props.bg = c; return self end
      function ElementBuilder:color(c) self.props.text_color = c; return self end
      function ElementBuilder:p(v) self.props.p = v; return self end
      function ElementBuilder:px(v) self.props.px = v; return self end
      function ElementBuilder:py(v) self.props.py = v; return self end
      function ElementBuilder:m(v) self.props.m = v; return self end
      function ElementBuilder:mx(v) self.props.mx = v; return self end
      function ElementBuilder:my(v) self.props.my = v; return self end
      function ElementBuilder:rounded(r) self.props.rounded = r; return self end
      function ElementBuilder:border(b) self.props.border = b; return self end
      function ElementBuilder:border_color(c) self.props.border_color = c; return self end
      function ElementBuilder:gap(g) self.props.gap = g; return self end
      function ElementBuilder:flex_row() self.props.flex_row = true; return self end
      function ElementBuilder:flex_col() self.props.flex_col = true; return self end
      function ElementBuilder:items_center() self.props.items_center = true; return self end
      function ElementBuilder:justify_center() self.props.justify_center = true; return self end
      function ElementBuilder:shadow_md() self.props.shadow_md = true; return self end
      function ElementBuilder:bold() self.props.bold = true; return self end
      function ElementBuilder:size(s) self.props.font_size = s; return self end
      function ElementBuilder:font_family(name) self.props.font_family = name; return self end
      function ElementBuilder:on_click(fn)
        local id = __next_click_id
        __next_click_id = __next_click_id + 1
        __click_callbacks[id] = fn
        self.click_id = id
        return self
      end

      function ElementBuilder:serialize()
        local ch_nodes = {}
        for _, c in ipairs(self.children) do
          if type(c) == 'table' and c.serialize then
            table.insert(ch_nodes, c:serialize())
          end
        end

        local p = self.props
        return {
          kind = self.kind,
          text = self.text_content,
          label = p.label,
          bg = p.bg,
          textColor = p.text_color or p.color,
          borderColor = p.border_color,
          border = p.border,
          rounded = p.rounded,
          p = p.p,
          px = p.px,
          py = p.py,
          m = p.m,
          mx = p.mx,
          my = p.my,
          w = p.w,
          h = p.h,
          wFull = p.w_full,
          hFull = p.h_full,
          gap = p.gap,
          flexRow = p.flex_row,
          flexCol = p.flex_col,
          itemsCenter = p.items_center,
          justifyCenter = p.justify_center,
          fontSize = p.font_size,
          fontFamily = p.font_family,
          bold = p.bold,
          variant = p.variant,
          disabled = p.disabled,
          onClickId = self.click_id,
          cursorPointer = (self.click_id ~= nil or p.cursor_pointer == true),
          children = ch_nodes
        }
      end

      ui = {}
      function ui.div(p) return ElementBuilder.new('div', p) end
      function ui.text(s)
        local b = ElementBuilder.new('text', {})
        b.text_content = tostring(s)
        return b
      end
      function ui.row(p)
        p = p or {}
        p.flex_row = true
        return ElementBuilder.new('row', p)
      end
      ui.Row = ui.row
      function ui.col(p)
        p = p or {}
        p.flex_col = true
        return ElementBuilder.new('col', p)
      end
      ui.Col = ui.col
      ui.stack = ui.col
      ui.Stack = ui.col
      function ui.Button(p)
        p = p or {}
        p.label = p.label or p[1] or 'Button'
        return ElementBuilder.new('button', p)
      end
      function ui.Card(p)
        p = p or {}
        local c = ElementBuilder.new('card', p)
        if p.title then c:child(ui.text(p.title):bold():size(18)) end
        if p.subtitle then c:child(ui.text(p.subtitle):size(13)) end
        if p.children then for _, ch in ipairs(p.children) do c:child(ch) end end
        return c
      end
      function ui.Badge(p)
        p = p or {}
        return ElementBuilder.new('badge', {
          label = p.text or p[1] or 'Badge',
          textColor = p.color,
          bg = p.bg,
          px = 8, py = 3, rounded = 999
        })
      end
      function ui.load_font(path)
        return path:match("([^/\\]+)%.%w+$") or path
      end
      ui.add_font = ui.load_font
      font = { load = ui.load_font }

      -- User code
      ${userCode}

      if type(App) == 'function' then
        if __trigger_click then
          local _ = App()
          if __click_callbacks[__trigger_click] then
            __click_callbacks[__trigger_click]()
          end
        end

        __click_callbacks = {}
        __next_click_id = 1
        local root = App()
        if root and root.serialize then
          print('__GPUI_TREE__:' .. to_json_val(root:serialize()))
          print('__SIGNALS__:' .. to_json_val(__stored_signals))
        end
      end
    `;
  }

  public async runLuaCode(code: string): Promise<SerializedNode | null> {
    await this.init();
    this.lastUserCode = code;
    this.lastRuntimeError = null;


    const harness = this.buildHarness(code);

    try {
      const res = run(harness);
      return this.processRunResult(res);
    } catch (err: unknown) {
      const msg = this.lastRuntimeError || (err instanceof Error ? err.message : String(err));
      this.emitLog('error', `Runtime Error: ${msg}`);
      throw new Error(msg);
    }
  }

  public async triggerClick(onClickId: number): Promise<SerializedNode | null> {
    if (!this.lastUserCode) return null;
    await this.init();
    this.lastRuntimeError = null;

    const harness = this.buildHarness(this.lastUserCode, onClickId);

    try {
      const res = run(harness);
      return this.processRunResult(res);
    } catch (err: unknown) {
      const msg = this.lastRuntimeError || (err instanceof Error ? err.message : String(err));
      this.emitLog('error', `Click Error: ${msg}`);
      throw new Error(msg);
    }
  }

  private processRunResult(res: any): SerializedNode | null {
    if (res.error) {
      this.emitLog('error', res.error);
      throw new Error(res.error);
    }

    const lines = res.output ? res.output.split('\n') : [];
    let treeNode: SerializedNode | null = null;

    for (const line of lines) {
      if (line.startsWith('__GPUI_TREE__:')) {
        try {
          const jsonStr = line.slice('__GPUI_TREE__:'.length);
          treeNode = JSON.parse(jsonStr) as SerializedNode;
          const normalize = (n: any) => {
            if (!n || typeof n !== 'object') return;
            if (!Array.isArray(n.children)) {
              n.children = [];
            }
            for (const ch of n.children) {
              normalize(ch);
            }
          };
          normalize(treeNode);
        } catch (e: any) {
          this.emitLog('error', `Failed to parse UI tree JSON: ${e.message}`);
        }
      } else if (line.startsWith('__SIGNALS__:')) {
        try {
          const jsonStr = line.slice('__SIGNALS__:'.length);
          const newSignals = JSON.parse(jsonStr) as Record<string, any>;
          for (const [k, v] of Object.entries(newSignals)) {
            if (this.signals[k] !== v) {
              this.signals[k] = v;
              if (this.onSignal) {
                this.onSignal(k, v);
              }
            }
          }
        } catch {
          // ignore signal parse error
        }
      } else if (line.trim().length > 0) {
        this.emitLog('info', line);
      }
    }

    return treeNode;
  }
}
