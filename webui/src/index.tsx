if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { render, hydrate } from 'preact';
import { CurrentUserData } from './apitypes';
import App from './components/App';

if (process.env.NODE_ENV === 'development') {
    while (document.body.firstChild) {
        document.body.firstChild.remove();
    }
    fetch('/api/bot')
        .then(async (response) => {
            if (!response.ok) {
                throw new Error(`${response.status} ${response.statusText}`);
            }
            const bot: CurrentUserData = await response.json();
            render(<App bot={bot} />, document.body);
        })
        .catch((err) => {
            console.error(err);
        });
} else {
    const botData = document.head.querySelector<HTMLScriptElement>('script[type="application/x-bot-user+json"]');
    const bot: CurrentUserData = JSON.parse(botData?.textContent!);
    hydrate(<App bot={bot} />, document.body);
}
