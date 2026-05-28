import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en";
import tr from "./locales/tr";
import es from "./locales/es";
import de from "./locales/de";
import fr from "./locales/fr";
import ja from "./locales/ja";
import zh from "./locales/zh";
import ru from "./locales/ru";
import pt from "./locales/pt";
import ar from "./locales/ar";

export const SUPPORTED_LANGS = [
  { code: "en", name: "English" },
  { code: "tr", name: "Türkçe" },
  { code: "es", name: "Español" },
  { code: "de", name: "Deutsch" },
  { code: "fr", name: "Français" },
  { code: "ja", name: "日本語" },
  { code: "zh", name: "中文" },
  { code: "ru", name: "Русский" },
  { code: "pt", name: "Português" },
  { code: "ar", name: "العربية" },
] as const;

const resources = {
  en: { translation: en },
  tr: { translation: tr },
  es: { translation: es },
  de: { translation: de },
  fr: { translation: fr },
  ja: { translation: ja },
  zh: { translation: zh },
  ru: { translation: ru },
  pt: { translation: pt },
  ar: { translation: ar },
} as const;

const STORAGE_KEY = "lanblaze.lang";
const RTL = new Set(["ar"]);

function detect(): string {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && stored in resources) return stored;
  const navLang = (navigator.language ?? "en").split("-")[0];
  return navLang in resources ? navLang : "en";
}

const initial = detect();

i18n.use(initReactI18next).init({
  resources,
  lng: initial,
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

document.documentElement.lang = initial;
document.documentElement.dir = RTL.has(initial) ? "rtl" : "ltr";

export function setLang(code: string) {
  i18n.changeLanguage(code);
  try { localStorage.setItem(STORAGE_KEY, code); } catch {}
  document.documentElement.lang = code;
  document.documentElement.dir = RTL.has(code) ? "rtl" : "ltr";
}

export default i18n;
