if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { render } from 'preact';
import UIkit from 'uikit';
import Icons from 'uikit/dist/js/uikit-icons';
import App, { CurrentUserData } from './components/App';

UIkit.use(Icons);

const botData = document.head.querySelector('script[type="application/x-bot-user+json"]');
const bot: CurrentUserData = JSON.parse(botData!.textContent!);

render(<App bot={bot} />, document.body);
