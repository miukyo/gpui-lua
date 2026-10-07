import { defineConfig } from 'vitepress';

const base = process.env.GITHUB_PAGES ? '/gpui.lua/' : '/';

export default defineConfig({
  base,
  title: 'gpui.lua',
  description: 'Build GPU-accelerated UI with Rust & Lua',
  cleanUrls: true,
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: `${base}logo.svg` }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    siteTitle: '',
    nav: [
      { text: 'Guide', link: '/guide/getting-started', activeMatch: '/guide/' },
      { text: 'API Reference', link: '/api/lua-api', activeMatch: '/api/' },
      {
        text: 'Playground',
        link: '/playground/',
        activeMatch: '/playground/',
      },
      { text: 'GitHub', link: 'https://github.com/miukyo/gpui-lua' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          collapsed: false,
          items: [
            { text: 'Two Workflows (Lua / Rust)', link: '/guide/getting-started' },
            { text: 'Configuration File (gpui.toml)', link: '/guide/config-file' },
            { text: 'Single-Binary Packaging', link: '/guide/embedded-assets' },
            { text: 'Live Hot Reloading', link: '/guide/hot-reloading' },
          ],
        },
        {
          text: 'UI & Layout',
          collapsed: false,
          items: [
            { text: 'DSL & Styling Properties', link: '/guide/dsl-and-styling' },
            { text: 'Client-Side Window Chrome (CSD)', link: '/guide/csd-and-window' },
            { text: 'Reactive State & Signals', link: '/guide/reactive-state' },
          ],
        },
        {
          text: 'Rust Integration',
          collapsed: false,
          items: [
            { text: 'Rust Host & Backend Bridge', link: '/guide/rust-backend' },
            { text: 'JSON RPC Functions', link: '/guide/json-rpc' },
          ],
        },
        {
          text: 'Standard Library',
          collapsed: false,
          items: [
            { text: 'Standard Library Overview', link: '/guide/stdlib' },
            { text: 'Filesystem (fs)', link: '/guide/stdlib/fs' },
            { text: 'HTTP & Net (http, net)', link: '/guide/stdlib/http' },
            { text: 'Hardware Media (media)', link: '/guide/stdlib/media' },
            { text: 'WebRTC P2P (webrtc)', link: '/guide/stdlib/webrtc' },
            { text: 'Audio Engine (audio)', link: '/guide/stdlib/audio' },
            { text: 'Timers (timer)', link: '/guide/stdlib/timer' },
            { text: 'Operating System (os)', link: '/guide/stdlib/os' },
            { text: 'Database (db)', link: '/guide/stdlib/db' },
            { text: 'JSON & Utilities (json)', link: '/guide/stdlib/json' },
            { text: 'Crypto & FFI (crypto, ffi)', link: '/guide/stdlib/crypto-ffi' },
          ],
        },
      ],
      '/api/': [
        {
          text: 'API Reference',
          collapsed: false,
          items: [
            { text: 'Complete Lua Reference', link: '/api/lua-api' },
            { text: 'UI Elements', link: '/api/ui-elements' },
            { text: 'Layout Containers', link: '/api/layout-containers' },
            { text: 'Builder Style Methods', link: '/api/builder-methods' },
            { text: 'Reactive State API', link: '/api/reactive-api' },
            { text: 'Hardware Media API', link: '/api/media-api' },
            { text: 'WebRTC P2P API', link: '/api/webrtc-api' },
            { text: 'CLI Commands', link: '/api/cli-commands' },
            { text: 'Rust Host API (LuaApp, CSD)', link: '/api/rust-api' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'Reference',
          collapsed: false,
          items: [
            { text: 'Lua API Reference', link: '/reference/lua-api' },
            { text: 'Rust API Reference', link: '/reference/rust-api' },
          ],
        },
      ],
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/miukyo/gpui-lua' },
    ],
    footer: {
      message: 'Released under the MIT and Apache 2.0 Licenses.',
      copyright: '',
    },
    search: {
      provider: 'local',
    },
  },
  vite: {
    assetsInclude: ['**/*.wasm'],
    // The playground editor is fed straight from crates/gpui_lua/examples/*.lua.
    server: {
      fs: {
        allow: ['..'],
      },
    },
    optimizeDeps: {
      exclude: [],
    },
    build: {
      target: 'esnext',
    },
  },
});
