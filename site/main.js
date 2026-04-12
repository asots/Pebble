// -- i18n translations --
const i18n = {
  en: {
    // Nav
    navPrivacy: "Privacy",
    navFeatures: "Features",
    navPreview: "Preview",
    navGitHub: "GitHub",
    navDownload: "Download",
    // Hero
    heroEyebrow: "Open Source \u00b7 AGPL-3.0",
    heroTitle: 'Email that stays <em>yours</em>',
    heroDesc: "A beautifully crafted, privacy-first desktop email client.<br>Mail, search, and attachments stay on your device.",
    heroBtnDownload: "Download for Free",
    heroBtnSource: "View Source",
    heroNote: "Available for Windows, macOS, and Linux",
    heroImgPlaceholder: "Replace with main screenshot",
    // Privacy
    privacyLabel: "Privacy First",
    privacyTitle: "Your inbox is nobody else\u2019s business",
    privacyDesc: "Everything stored locally. No cloud, no telemetry, no third-party access.",
    privacyEncTitle: "AES-256 Encrypted",
    privacyEncDesc: "OAuth tokens and credentials encrypted at rest with a per-device key.",
    privacyLocalTitle: "100% Local Storage",
    privacyLocalDesc: "SQLite database, search index, and attachments \u2014 all on your device.",
    privacyOpenTitle: "Open Source",
    privacyOpenDesc: "AGPL-3.0 licensed. Inspect every line of code yourself.",
    // Features
    featuresLabel: "Features",
    featuresTitle: "Everything you need, nothing you don\u2019t",
    featuresDesc: "Thoughtfully designed around how you actually work with email.",
    featKanbanTitle: "Kanban Board",
    featKanbanDesc: "Drag emails across Todo, Waiting, and Done. Turn your inbox into a task board.",
    featSearchTitle: "Smart Search",
    featSearchDesc: "Full-text search powered by Tantivy. Find anything by content, sender, or date.",
    featMultiTitle: "Multi-Provider",
    featMultiDesc: "Gmail, Outlook, and any IMAP server. All accounts in one place.",
    featSnoozeTitle: "Snooze & Star",
    featSnoozeDesc: "Snooze to resurface later. Star what matters. Stay focused.",
    featTransTitle: "Translation",
    featTransDesc: "Built-in bilingual view with DeepL or LLM. Read emails in any language.",
    featRulesTitle: "Rules Engine",
    featRulesDesc: "Auto-label, move, or flag emails with custom conditions and actions.",
    featKeyboardTitle: "Keyboard Friendly",
    featKeyboardDesc: "Shortcuts for navigation, compose, reply, archive, and search — fully customizable.",
    featThemeTitle: "Dark & Light",
    featThemeDesc: "Beautiful themes with system-aware auto-switching.",
    featI18nTitle: "i18n Ready",
    featI18nDesc: "English and Chinese built-in. Easily extensible for more languages.",
    // Preview
    previewLabel: "Preview",
    previewTitle: "See it in action",
    previewDesc: "Crafted with care for every pixel and interaction.",
    previewInbox: "<strong>Inbox</strong> \u2014 Clean three-panel layout",
    previewKanban: "<strong>Kanban</strong> \u2014 Drag-and-drop email management",
    previewDark: "<strong>Dark Mode</strong> \u2014 Gorgeous dark interface",
    previewSettings: "<strong>Privacy</strong> \u2014 Your data stays on your device",
    previewInboxAlt: "Inbox screenshot",
    previewKanbanAlt: "Kanban screenshot",
    previewDarkAlt: "Dark mode screenshot",
    previewSettingsAlt: "Settings screenshot",
    // Tech
    techLabel: "Tech Stack",
    techTitle: "Built on solid ground",
    techDesc: "Modern, performant technologies chosen for reliability.",
    techRust: "Backend core",
    techTauri: "Desktop framework",
    techReact: "Frontend UI",
    techSQLite: "Local database",
    techTantivy: "Full-text search",
    // CTA
    ctaTitle: "Ready to take back your inbox?",
    ctaDesc: "Free and open source, forever.",
    ctaBtnDownload: "Download Latest Release",
    // Footer
    footerCopyright: "\u00a9 2026 Pebble. Released under the GNU Affero General Public License v3.0.",
  },
  zh: {
    navPrivacy: "\u9690\u79c1\u4fdd\u62a4",
    navFeatures: "\u529f\u80fd\u7279\u6027",
    navPreview: "\u9884\u89c8",
    navGitHub: "GitHub",
    navDownload: "\u4e0b\u8f7d",
    heroEyebrow: "\u5f00\u6e90 \u00b7 AGPL-3.0",
    heroTitle: '\u90ae\u4ef6\uff0c\u59cb\u7ec8<em>\u5c5e\u4e8e\u4f60</em>',
    heroDesc: "\u7cbe\u5fc3\u6253\u9020\u7684\u9690\u79c1\u4f18\u5148\u684c\u9762\u90ae\u4ef6\u5ba2\u6237\u7aef\u3002<br>\u90ae\u4ef6\u3001\u641c\u7d22\u4e0e\u9644\u4ef6\u59cb\u7ec8\u4fdd\u5b58\u5728\u4f60\u7684\u8bbe\u5907\u4e0a\u3002",
    heroBtnDownload: "\u514d\u8d39\u4e0b\u8f7d",
    heroBtnSource: "\u67e5\u770b\u6e90\u7801",
    heroNote: "\u652f\u6301 Windows\u3001macOS \u548c Linux",
    heroImgPlaceholder: "\u66ff\u6362\u4e3b\u622a\u56fe",
    privacyLabel: "\u9690\u79c1\u4f18\u5148",
    privacyTitle: "\u4f60\u7684\u6536\u4ef6\u7bb1\u4e0d\u5173\u4efb\u4f55\u4eba\u7684\u4e8b",
    privacyDesc: "\u6240\u6709\u6570\u636e\u672c\u5730\u5b58\u50a8\u3002\u65e0\u4e91\u7aef\u3001\u65e0\u9065\u6d4b\u3001\u65e0\u7b2c\u4e09\u65b9\u8bbf\u95ee\u3002",
    privacyEncTitle: "AES-256 \u52a0\u5bc6",
    privacyEncDesc: "OAuth \u4ee4\u724c\u548c\u51ed\u8bc1\u4f7f\u7528\u8bbe\u5907\u5bc6\u94a5\u52a0\u5bc6\u5b58\u50a8\u3002",
    privacyLocalTitle: "100% \u672c\u5730\u5b58\u50a8",
    privacyLocalDesc: "SQLite \u6570\u636e\u5e93\u3001\u641c\u7d22\u7d22\u5f15\u548c\u9644\u4ef6 \u2014 \u5168\u90e8\u5728\u4f60\u7684\u8bbe\u5907\u4e0a\u3002",
    privacyOpenTitle: "\u5f00\u653e\u6e90\u7801",
    privacyOpenDesc: "AGPL-3.0 \u8bb8\u53ef\u8bc1\u3002\u4f60\u53ef\u4ee5\u5ba1\u67e5\u6bcf\u4e00\u884c\u4ee3\u7801\u3002",
    featuresLabel: "\u529f\u80fd\u7279\u6027",
    featuresTitle: "\u4f60\u9700\u8981\u7684\u4e00\u5207\uff0c\u4ec5\u6b64\u800c\u5df2",
    featuresDesc: "\u56f4\u7ed5\u4f60\u5b9e\u9645\u7684\u90ae\u4ef6\u5de5\u4f5c\u6d41\u7cbe\u5fc3\u8bbe\u8ba1\u3002",
    featKanbanTitle: "\u770b\u677f\u89c6\u56fe",
    featKanbanDesc: "\u62d6\u62fd\u90ae\u4ef6\u5230\u5f85\u529e\u3001\u7b49\u5f85\u548c\u5df2\u5b8c\u6210\u3002\u628a\u6536\u4ef6\u7bb1\u53d8\u6210\u4efb\u52a1\u677f\u3002",
    featSearchTitle: "\u667a\u80fd\u641c\u7d22",
    featSearchDesc: "\u57fa\u4e8e Tantivy \u7684\u5168\u6587\u641c\u7d22\u3002\u6309\u5185\u5bb9\u3001\u53d1\u4ef6\u4eba\u6216\u65e5\u671f\u67e5\u627e\u4efb\u4f55\u90ae\u4ef6\u3002",
    featMultiTitle: "\u591a\u8d26\u6237\u652f\u6301",
    featMultiDesc: "Gmail\u3001Outlook \u548c\u4efb\u4f55 IMAP \u670d\u52a1\u5668\u3002\u6240\u6709\u8d26\u6237\u96c6\u4e2d\u7ba1\u7406\u3002",
    featSnoozeTitle: "\u8d2a\u7761\u4e0e\u661f\u6807",
    featSnoozeDesc: "\u8d2a\u7761\u4ee5\u7a0d\u540e\u91cd\u65b0\u663e\u793a\u3002\u661f\u6807\u6807\u8bb0\u91cd\u8981\u90ae\u4ef6\u3002\u4fdd\u6301\u4e13\u6ce8\u3002",
    featTransTitle: "\u7ffb\u8bd1\u529f\u80fd",
    featTransDesc: "\u5185\u7f6e\u53cc\u8bed\u89c6\u56fe\uff0c\u652f\u6301 DeepL \u6216 LLM\u3002\u7528\u4efb\u4f55\u8bed\u8a00\u9605\u8bfb\u90ae\u4ef6\u3002",
    featRulesTitle: "\u89c4\u5219\u5f15\u64ce",
    featRulesDesc: "\u81ea\u52a8\u6807\u8bb0\u3001\u79fb\u52a8\u6216\u6807\u8bb0\u90ae\u4ef6\u3002\u81ea\u5b9a\u4e49\u6761\u4ef6\u548c\u64cd\u4f5c\u3002",
    featKeyboardTitle: "\u952e\u76d8\u53cb\u597d",
    featKeyboardDesc: "\u5bfc\u822a\u3001\u64b0\u5199\u3001\u56de\u590d\u3001\u5f52\u6863\u548c\u641c\u7d22\u90fd\u6709\u5feb\u6377\u952e\uff0c\u4e14\u53ef\u81ea\u5b9a\u4e49\u3002",
    featThemeTitle: "\u6df1\u8272\u4e0e\u6d45\u8272",
    featThemeDesc: "\u7cbe\u7f8e\u7684\u4e3b\u9898\uff0c\u652f\u6301\u8ddf\u968f\u7cfb\u7edf\u81ea\u52a8\u5207\u6362\u3002",
    featI18nTitle: "\u591a\u8bed\u8a00\u652f\u6301",
    featI18nDesc: "\u5185\u7f6e\u4e2d\u82f1\u6587\u652f\u6301\u3002\u53ef\u8f7b\u677e\u6269\u5c55\u66f4\u591a\u8bed\u8a00\u3002",
    previewLabel: "\u9884\u89c8",
    previewTitle: "\u5b9e\u9645\u6548\u679c\u5c55\u793a",
    previewDesc: "\u7528\u5fc3\u6253\u78e8\u6bcf\u4e00\u4e2a\u50cf\u7d20\u548c\u4ea4\u4e92\u3002",
    previewInbox: "<strong>\u6536\u4ef6\u7bb1</strong> \u2014 \u7b80\u6d01\u7684\u4e09\u680f\u5e03\u5c40",
    previewKanban: "<strong>\u770b\u677f</strong> \u2014 \u62d6\u62fd\u5f0f\u90ae\u4ef6\u7ba1\u7406",
    previewDark: "<strong>\u6df1\u8272\u6a21\u5f0f</strong> \u2014 \u7cbe\u7f8e\u7684\u6df1\u8272\u754c\u9762",
    previewSettings: "<strong>\u9690\u79c1\u4fdd\u62a4</strong> \u2014 \u4f60\u7684\u6570\u636e\u59cb\u7ec8\u7559\u5728\u672c\u5730",
    previewInboxAlt: "\u6536\u4ef6\u7bb1\u622a\u56fe",
    previewKanbanAlt: "\u770b\u677f\u622a\u56fe",
    previewDarkAlt: "\u6df1\u8272\u6a21\u5f0f\u622a\u56fe",
    previewSettingsAlt: "\u8bbe\u7f6e\u622a\u56fe",
    techLabel: "\u6280\u672f\u6808",
    techTitle: "\u575a\u5b9e\u7684\u6280\u672f\u57fa\u7840",
    techDesc: "\u73b0\u4ee3\u3001\u9ad8\u6027\u80fd\u7684\u6280\u672f\u9009\u578b\uff0c\u4ee5\u53ef\u9760\u6027\u4e3a\u6838\u5fc3\u3002",
    techRust: "\u540e\u7aef\u6838\u5fc3",
    techTauri: "\u684c\u9762\u6846\u67b6",
    techReact: "\u524d\u7aef UI",
    techSQLite: "\u672c\u5730\u6570\u636e\u5e93",
    techTantivy: "\u5168\u6587\u641c\u7d22",
    ctaTitle: "\u51c6\u5907\u597d\u62ff\u56de\u4f60\u7684\u6536\u4ef6\u7bb1\u4e86\u5417\uff1f",
    ctaDesc: "\u514d\u8d39\u5f00\u6e90\uff0c\u6c38\u8fdc\u5982\u6b64\u3002",
    ctaBtnDownload: "\u4e0b\u8f7d\u6700\u65b0\u7248\u672c",
    footerCopyright: "\u00a9 2026 Pebble\u3002\u57fa\u4e8e GNU Affero \u901a\u7528\u516c\u5171\u8bb8\u53ef\u8bc1 v3.0 \u53d1\u5e03\u3002",
  }
};

// -- Language switching --
let currentLang = localStorage.getItem('pebble-site-lang') || 'en';

function setLang(lang) {
  currentLang = lang;
  localStorage.setItem('pebble-site-lang', lang);
  document.documentElement.lang = lang;

  const t = i18n[lang];

  // Update all elements with data-i18n attribute
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.getAttribute('data-i18n');
    if (t[key] !== undefined) {
      if (el.hasAttribute('data-i18n-html')) {
        el.innerHTML = t[key];
      } else {
        el.textContent = t[key];
      }
    }
  });

  // Update lang switcher active state
  document.querySelectorAll('.lang-switch button').forEach(btn => {
    btn.classList.toggle('active', btn.getAttribute('data-lang') === lang);
  });
}

// -- Scroll reveal observer --
const observer = new IntersectionObserver(entries => {
  entries.forEach(e => { if (e.isIntersecting) e.target.classList.add('show'); });
}, { threshold: 0.1, rootMargin: '0px 0px -32px 0px' });
document.querySelectorAll('.reveal').forEach(el => observer.observe(el));

// -- Nav scroll effect --
window.addEventListener('scroll', () => {
  document.getElementById('nav').classList.toggle('scrolled', scrollY > 32);
}, { passive: true });

// -- Language switcher event --
document.querySelectorAll('.lang-switch button').forEach(btn => {
  btn.addEventListener('click', () => setLang(btn.getAttribute('data-lang')));
});

// -- Initialize language --
setLang(currentLang);
