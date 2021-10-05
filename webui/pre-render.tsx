import * as fs from 'fs';
import { promisify } from 'util';
import * as glob from 'glob';
import { render } from 'preact-render-to-string';
import { CurrentUserData } from './src/apitypes';
import App from './src/components/App';

async function renderIndex() {
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

    let html = await fs.promises.readFile('./src/index.template.html', 'utf8');
    html = html.replace(/<body>[\s\S]*<\/body>/, `<body>${body}</body>`);
    await fs.promises.writeFile('./src/index.html', html, 'utf8');
}

async function renderIcons() {
    const fnames = await promisify(glob)('./src/**/*.tsx', { nosort: true });
    const icons = await Promise.all(
        fnames.map(async (fname) => {
            const source = await fs.promises.readFile(fname, 'utf8');
            return Array.from(source.matchAll(/uk-icon="([^"]+)"/g), (match) => match[1]);
        }),
    );
    const uniqueIcons = Array.from(new Set(icons.flat()));

    const js = `
        import UIkit from 'uikit';
        UIkit.icon.add({
            ${uniqueIcons
                .map((icon) => `'${icon}': require('bundle-text:uikit/src/images/icons/${icon}.svg'),`)
                .join('\n')}
        });
    `;
    await fs.promises.writeFile('./src/uikit.ts', js, 'utf8');
}

Promise.all([renderIndex(), renderIcons()]).catch((err) => {
    console.error(err);
    process.exit(1);
});
