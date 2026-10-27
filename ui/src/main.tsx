import React from 'react';
import ReactDOM from 'react-dom/client';
import { IntlProvider } from './i18n';
import App from './App';
import './styles.css';
import { messages, locale } from './i18n';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <IntlProvider locale={locale} defaultLocale="en" messages={messages}>
      <App />
    </IntlProvider>
  </React.StrictMode>,
);
