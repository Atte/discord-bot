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

async function mapGlobFiles<T>(pattern: string, map: (source: string) => T[]): Promise<T[]> {
    const fnames = await promisify(glob)(pattern, { nosort: true });
    const resultSets = await Promise.all(fnames.map(async (fname) => map(await fs.promises.readFile(fname, 'utf8'))));
    return resultSets.flat();
}

async function renderUikitScript() {
    const icons = await mapGlobFiles('./src/**/*.tsx', (source) =>
        Array.from(source.matchAll(/uk-icon="([^"]+)"/g), (match) => match[1]),
    );
    const js = `
        import UIkit from 'uikit';
        UIkit.icon.add({
            ${Array.from(new Set(icons))
                .map((icon) => `'${icon}': require('bundle-text:uikit/src/images/icons/${icon}.svg'),`)
                .join('\n')}
        });
    `;
    await fs.promises.writeFile('./src/uikit.ts', js, 'utf8');
}

async function renderUikitStyle() {
    const components = await mapGlobFiles('./node_modules/uikit/src/less/components/_import.less', (source) =>
        Array.from(source.matchAll(/@import "([^.]+)\.less";/g)).map((match) => match[1]),
    );
    const themes = await mapGlobFiles('./node_modules/uikit/src/less/theme/_import.less', (source) =>
        Array.from(source.matchAll(/@import "([^.]+)\.less";/g)).map((match) => match[1]),
    );
    const both = Array.from(new Set(components.concat(themes)));

    const used = new Set(
        ['variables', 'mixin', 'base', 'utility', 'inverse'].concat(
            await mapGlobFiles('./src/**/*.tsx', (source) => {
                return both.filter((name) => new RegExp(`[^a-z]uk-${name}[^a-z]`).test(source));
            }),
        ),
    );

    // TODO: make a proper dependency solver
    used.add('nav');

    const usedComponents = components
        .filter((name) => used.has(name))
        .map((name) => `@import "uikit/src/less/components/${name}.less";`);
    const usedThemes = themes
        .filter((name) => used.has(name))
        .map((name) => `@import "uikit/src/less/theme/${name}.less";`);

    await fs.promises.writeFile('./src/uikit.less', usedComponents.concat(usedThemes).join('\n'), 'utf8');
}

Promise.all([renderIndex(), renderUikitScript(), renderUikitStyle()]).catch((err) => {
    console.error(err);
    process.exit(1);
});
