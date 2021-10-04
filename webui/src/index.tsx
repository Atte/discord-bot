if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { hydrate } from 'preact';
import UIkit from 'uikit';
import Icons from 'uikit/dist/js/uikit-icons';
import { CurrentUserData } from './apitypes';
import App from './components/App';

UIkit.use(Icons);

const botData = document.head.querySelector('script[type="application/x-bot-user+json"]');
const bot: CurrentUserData = JSON.parse(botData!.textContent!);

hydrate(<App bot={bot} />, document.body);
