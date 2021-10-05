if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { hydrate } from 'preact';
import { CurrentUserData } from './apitypes';
import App from './components/App';

const botData = document.head.querySelector('script[type="application/x-bot-user+json"]');
const bot: CurrentUserData = JSON.parse(botData!.textContent!);

hydrate(<App bot={bot} />, document.body);
