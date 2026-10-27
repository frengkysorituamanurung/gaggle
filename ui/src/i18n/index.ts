import React from 'react';
import { defineMessages, useIntl, IntlProvider } from 'react-intl';
import type { IntlShape, MessageDescriptor } from 'react-intl';
import enMessages from './messages/en.json';

// Re-export react-intl utilities that components use directly.
export { defineMessages, useIntl, IntlProvider };
export type { IntlShape, MessageDescriptor };

/** The default locale used throughout the app. */
export const defaultLocale = 'en';
export const locale = 'en';

/** English message catalog. */
export const messages = enMessages;
export { enMessages };

/** Locale helpers (simplified: English only). */
export function getLocale(): { locale: string; messageLocale: string } {
  return { locale: 'en', messageLocale: 'en' };
}

export const currentLocale = 'en';
export const currentMessageLocale = 'en';

export async function loadMessages(_locale?: string): Promise<Record<string, string>> {
  return enMessages as Record<string, string>;
}

export function I18nProvider({ children }: { children: React.ReactNode }): React.ReactElement {
  return React.createElement(
    IntlProvider,
    { locale: 'en', defaultLocale: 'en', messages: enMessages },
    children
  );
}
