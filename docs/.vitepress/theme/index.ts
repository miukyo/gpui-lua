import DefaultTheme from 'vitepress/theme';
import type { Theme } from 'vitepress';
import { defineClientComponent } from 'vitepress';
import './style.css';

const PlaygroundClient = defineClientComponent(() => {
  return import('./components/Playground.vue');
});

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('Playground', PlaygroundClient);
  }
} satisfies Theme;
