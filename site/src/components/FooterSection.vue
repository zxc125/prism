<script setup lang="ts">
// Footer（方案 §6.8 第 12 节）
// 签名元素：收束棱镜分光 beam + wordmark + 链接列 + 许可声明
import { computed } from "vue";
import PrismLogo from "./PrismLogo.vue";
import GithubIcon from "./GithubIcon.vue";
import { useLang } from "../composables/useLang";
import { withBase } from "vitepress";

const { t } = useLang();

const cols = computed(() => [
  {
    title: t("footer.col1.title"),
    links: [
      { label: t("footer.col1.l1"), href: "#diagnosis" },
      { label: t("footer.col1.l2"), href: "#multiwindow" },
      { label: t("footer.col1.l3"), href: "#deploy" },
      { label: t("footer.col1.l4"), href: "#compare" },
    ],
  },
  {
    title: t("footer.col2.title"),
    links: [
      { label: t("footer.col2.docs"), href: withBase("/docs/quickstart") },
      { label: t("footer.col2.l1"), href: "#quickstart" },
      { label: t("footer.col2.l2"), href: "#architecture" },
      { label: t("footer.col2.l3"), href: "#manifesto" },
      { label: t("footer.col2.l4"), href: "#capabilities" },
    ],
  },
  {
    title: t("footer.col3.title"),
    links: [
      { label: "GitHub", href: "https://github.com/zxc125/prism" },
      { label: "rrweb", href: "https://github.com/rrweb-io/rrweb" },
      { label: "Tauri", href: "https://tauri.app" },
    ],
  },
]);
</script>

<template>
  <footer class="footer">
    <div class="footer-inner">
      <!-- 收束 beam -->
      <div class="footer-beam" aria-hidden="true">
        <span class="beam beam-amber" />
        <span class="beam beam-teal" />
      </div>

      <div class="footer-top">
        <div class="footer-brand">
          <div class="wordmark">
            <PrismLogo :height="32" />
            <span class="wordmark-text">{{ t("nav.wordmark") }}</span>
          </div>
          <p class="footer-tag">
            {{ t("footer.tag_pre") }}<br />
            {{ t("footer.tag_suf") }}
          </p>
          <a
            class="footer-gh"
            href="https://github.com/zxc125/prism"
            rel="noreferrer"
            target="_blank"
          >
            <GithubIcon :size="16" />
            <span>GitHub</span>
          </a>
        </div>

        <nav class="footer-cols">
          <div
            v-for="(c, i) in cols"
            :key="i"
            class="footer-col"
          >
            <h4 class="col-title mono">{{ c.title }}</h4>
            <ul class="col-list">
              <li v-for="(l, j) in c.links" :key="j">
                <a :href="l.href" :target="l.href.startsWith('http') || l.href.startsWith('/') ? '_blank' : undefined" rel="noreferrer">
                  {{ l.label }}
                </a>
              </li>
            </ul>
          </div>
        </nav>
      </div>

      <div class="footer-bottom">
        <span class="mono">{{ t("footer.copyright") }}</span>
        <span class="mono sep">·</span>
        <span class="mono">MIT License</span>
        <span class="mono sep">·</span>
        <span class="mono">{{ t("footer.built") }}</span>
        <span class="mono sep">·</span>
        <span class="mono">{{ t("footer.vuebits") }}</span>
      </div>
    </div>
  </footer>
</template>

<style scoped>
.footer {
  background: var(--color-ink);
  border-top: 1px solid var(--color-hair);
  padding: 3rem 2rem 2rem;
}
.footer-inner {
  max-width: 1180px;
  margin: 0 auto;
}
.footer-beam {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 3rem;
}
.beam {
  display: block;
  height: 2px;
  border-radius: 1px;
  flex: 1;
}
.beam-amber {
  background: var(--color-amber);
  box-shadow: 0 0 6px rgba(240, 168, 61, 0.4);
}
.beam-teal {
  background: var(--color-teal);
  box-shadow: 0 0 6px rgba(77, 208, 200, 0.4);
}
.footer-top {
  display: grid;
  grid-template-columns: 1fr;
  gap: 2.5rem;
  margin-bottom: 3rem;
}
@media (min-width: 768px) {
  .footer-top {
    grid-template-columns: 1.3fr 2fr;
  }
}
.wordmark {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  margin-bottom: 1rem;
}
.wordmark-text {
  font-size: 1rem;
  font-weight: 600;
  color: var(--color-bone);
}
.footer-tag {
  font-size: 0.875rem;
  line-height: 1.6;
  color: var(--color-ash);
  margin: 0 0 1.25rem;
  max-width: 22rem;
}
.footer-gh {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.8125rem;
  color: var(--color-bone);
  text-decoration: none;
  padding: 0.5rem 0.875rem;
  border: 1px solid var(--color-hair);
  border-radius: 6px;
  transition: all 0.15s;
}
.footer-gh:hover {
  border-color: var(--color-ash);
  background: var(--color-slate);
}
.footer-cols {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1.5rem;
}
@media (max-width: 480px) {
  .footer-cols {
    grid-template-columns: repeat(2, 1fr);
  }
}
.col-title {
  font-size: 0.6875rem;
  letter-spacing: 0.12em;
  color: var(--color-ash-deep);
  text-transform: uppercase;
  margin: 0 0 1rem;
}
.col-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
}
.col-list a {
  font-size: 0.875rem;
  color: var(--color-ash);
  text-decoration: none;
  transition: color 0.15s;
}
.col-list a:hover {
  color: var(--color-bone);
}
.footer-bottom {
  padding-top: 1.75rem;
  border-top: 1px solid var(--color-hair);
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.6875rem;
  color: var(--color-ash-deep);
  letter-spacing: 0.02em;
}
.sep {
  opacity: 0.5;
}
</style>
