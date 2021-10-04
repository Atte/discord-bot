import { render } from 'preact-render-to-string';
import { CurrentUserData } from './src/apitypes';
import App from './src/components/App';

const bot: CurrentUserData = {
    id: '(BOT_ID)',
    username: '(BOT_NAME)',
    avatar: '(BOT_AVATAR)',
    mfa_enabled: false,
    bot: true,
    discriminator: '(BOT_DISCRIMINATOR)' as unknown as number,
};

const html = render(
    <App bot={bot} />,
    {},
    {
        pretty: true,
    },
);

console.log(html);
