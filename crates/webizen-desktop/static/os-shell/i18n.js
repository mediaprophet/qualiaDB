/*
 * Small dependency-free localisation boundary for the OS shell.
 *
 * UI code asks for stable message ids rather than treating English copy as an
 * API. New language packs can be added without changing routes, storage, or
 * accessibility labels. Missing translations deliberately fall back to the
 * English source text; this is preferable to invented machine translations.
 */
(function (global) {
  "use strict";

  const messages = {
    en: {
      "savedItems.short": "Saved",
      "savedItems.title": "Saved Items",
      "savedItems.description": "Notes, saved records, and files.",
      "savedItems.save": "Save to Saved Items"
    }
  };

  function normaliseLocale(value) {
    const raw = String(value || "en").replace(/_/g, "-").toLowerCase();
    return raw.split("-")[0] || "en";
  }

  function selectedLocale() {
    try {
      return normaliseLocale(localStorage.getItem("webizen.locale") || navigator.language);
    } catch (_) {
      return "en";
    }
  }

  function t(key, fallback) {
    const locale = selectedLocale();
    return (messages[locale] && messages[locale][key]) || messages.en[key] || fallback || key;
  }

  function setLocale(locale) {
    const normalized = normaliseLocale(locale);
    try { localStorage.setItem("webizen.locale", normalized); } catch (_) {}
    document.documentElement.lang = normalized;
    document.dispatchEvent(new CustomEvent("webizen-locale-change", { detail: { locale: normalized } }));
    return normalized;
  }

  document.documentElement.lang = selectedLocale();
  global.WebizenI18n = Object.freeze({ t: t, setLocale: setLocale, locale: selectedLocale });
})(window);
