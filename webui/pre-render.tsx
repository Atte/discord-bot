import * as fs from 'fs';
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

const body = render(
    <App bot={bot} />,
    {},
    {
        pretty: true,
    },
);

let html = fs.readFileSync('./src/index.html', 'utf8');
html = html.replace(/<body>[\s\S]*<\/body>/, `<body>${body}</body>`);
fs.writeFileSync('./src/index.html', html, 'utf8');
