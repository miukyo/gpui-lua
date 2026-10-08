---
layout: home

hero:
  name: "GPUI.lua"
  text: "High-Performance GPU-Accelerated Lua Desktop Runtime"
  tagline: "Build native 60+ FPS desktop applications using LuaJIT and GPUI with Rust-grade performance, reactive state, and single-binary distribution."
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Interactive Playground
      link: /playground/
    - theme: alt
      text: API Reference
      link: /reference/lua-api

features:
  - title: Hardware-Accelerated Rendering
    details: Direct Metal (macOS), DirectX 12 / Vulkan (Windows), and Wayland/X11 (Linux) pipelines powered by GPUI-CE.
    icon: ⚡
  - title: Ultra-Fast LuaJIT Execution
    details: Native C-speed JIT compilation and memory management with zero IPC overhead between Rust and Lua.
    icon: 🚀
  - title: Declarative Tailwind-Style DSL
    details: Build component trees effortlessly with chaining methods, flexbox layout, and full Catppuccin color palettes.
    icon: 🎨
  - title: Reactive State with Hot-Reload
    details: SolidJS-style signal primitives with state preservation across script reloads during development.
    icon: 🔄
  - title: Native Window Backdrops
    details: Seamless support for Windows 11 Mica, Mica Alt, and Acrylic backdrops, plus macOS NSVisualEffectMaterial vibrancy.
    icon: 🪟
  - title: Single-Binary Distribution
    details: Bundle scripts and assets into self-extracting, standalone executables with zero external runtime dependencies.
    icon: 📦
---

<div style="margin-top: 40px;">
  <h2 style="font-size: 24px; font-weight: 700; margin-bottom: 12px; color: var(--vp-c-text-1);">Try GPUI.lua in Your Browser</h2>
  <p style="color: var(--vp-c-text-2); margin-bottom: 20px;">
    The live dual-engine playground runs the full Lua DSL inside a WebAssembly LuaJIT engine with live reactive state updates.
  </p>
  <Playground />
</div>
