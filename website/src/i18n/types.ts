import type { en } from "./locales/en";

/** Shape of every translation dictionary — derived from the English source. */
export type TranslationDict = typeof en;

export type Locale = "en" | "zh";
