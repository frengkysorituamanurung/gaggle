import React from 'react';
import { defineMessages, useIntl, IntlProvider } from 'react-intl';
import type { IntlShape, MessageDescriptor } from 'react-intl';
import enMessages from './messages/en.json';
import deMessages from './messages/de.json';
import esMessages from './messages/es.json';
import frMessages from './messages/fr.json';
import hiMessages from './messages/hi.json';
import idMessages from './messages/id.json';
import itMessages from './messages/it.json';
import jaMessages from './messages/ja.json';
import koMessages from './messages/ko.json';
import msMessages from './messages/ms.json';
import ptMessages from './messages/pt.json';
import ruMessages from './messages/ru.json';
import trMessages from './messages/tr.json';
import viMessages from './messages/vi.json';
import zhCNMessages from './messages/zh-CN.json';
import zhTWMessages from './messages/zh-TW.json';

// Re-export react-intl utilities that components use directly.
export { defineMessages, useIntl, IntlProvider };
export type { IntlShape, MessageDescriptor };

/** Supported locales matching Goose. */
export const supportedLocales = [
  'en', 'de', 'es', 'fr', 'hi', 'id', 'it', 'ja', 'ko', 'ms', 'pt', 'ru', 'tr', 'vi', 'zh-CN', 'zh-TW',
] as const;
export type SupportedLocale = (typeof supportedLocales)[number];

/** The default locale used throughout the app. */
export const defaultLocale = 'en';

const messageCatalogs: Record<string, Record<string, string>> = {
  en: enMessages as Record<string, string>,
  de: deMessages as Record<string, string>,
  es: esMessages as Record<string, string>,
  fr: frMessages as Record<string, string>,
  hi: hiMessages as Record<string, string>,
  id: idMessages as Record<string, string>,
  it: itMessages as Record<string, string>,
  ja: jaMessages as Record<string, string>,
  ko: koMessages as Record<string, string>,
  ms: msMessages as Record<string, string>,
  pt: ptMessages as Record<string, string>,
  ru: ruMessages as Record<string, string>,
  tr: trMessages as Record<string, string>,
  vi: viMessages as Record<string, string>,
  'zh-CN': zhCNMessages as Record<string, string>,
  'zh-TW': zhTWMessages as Record<string, string>,
};

/** Returns the browser-preferred locale, falling back to English. */
export function detectLocale(): SupportedLocale {
  if (typeof navigator === 'undefined') return 'en';
  const browserLocale = navigator.language;
  // Try exact match first (e.g., zh-CN)
  if ((supportedLocales as readonly string[]).includes(browserLocale)) {
    return browserLocale as SupportedLocale;
  }
  // Fall back to language code (e.g., zh)
  const langCode = browserLocale.split('-')[0].toLowerCase();
  const match = supportedLocales.find((l) => l.toLowerCase().startsWith(langCode));
  return match ?? 'en';
}

/** English message catalog (default). */
export const messages = enMessages;
export { enMessages };

/** Locale helpers. */
export function getLocale(): { locale: string; messageLocale: string } {
  const detected = detectLocale();
  return { locale: detected, messageLocale: detected };
}

export const currentLocale = detectLocale();
export const currentMessageLocale = currentLocale;

export async function loadMessages(locale?: string): Promise<Record<string, string>> {
  const resolved = locale && (supportedLocales as readonly string[]).includes(locale)
    ? locale
    : detectLocale();
  return messageCatalogs[resolved] ?? (enMessages as Record<string, string>);
}

export function I18nProvider({ children }: { children: React.ReactNode }): React.ReactElement {
  const detected = detectLocale();
  return React.createElement(
    IntlProvider,
    { locale: detected, defaultLocale: 'en', messages: messageCatalogs[detected] ?? enMessages },
    children
  );
}