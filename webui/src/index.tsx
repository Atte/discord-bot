import { render } from 'preact';
import UIkit from 'uikit';
import Icons from 'uikit/dist/js/uikit-icons';
import App, { CurrentUserData } from './components/App';

(UIkit.use as Function)(Icons);

let bot: CurrentUserData = JSON.parse((document.head.querySelector('script[type="application/x-bot-user+json"]') as HTMLScriptElement).textContent!);
render(<App bot={bot} />, document.body);
