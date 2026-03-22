import type { Locale, TranslationDict } from "./types";
import { en } from "./locales/en";
import { zh } from "./locales/zh";

export function useTranslations(locale: Locale): TranslationDict {
  return locale === "zh" ? zh : en;
}

/**
 * Build a page href for a given slug in a given locale.
 * @param slug  e.g. "demo.html", "demo/live.html", "" (home)
 */
export function pageHref(slug: string, locale: Locale, base: string): string {
  if (locale === "zh") {
    return slug ? `${base}zh/${slug}` : `${base}zh/index.html`;
  }
  return slug ? `${base}${slug}` : base;
}
