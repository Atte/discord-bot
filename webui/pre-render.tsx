require('module-alias/register');

import * as fs from 'fs';
import { promisify } from 'util';
import * as glob from 'glob';
import * as prettier from 'prettier';
import { render } from 'preact-render-to-string';
import App from './src/components/App';
import { GetBot, GetBot_bot } from './src/components/__generated__/GetBot';
import { ApolloClient, ApolloProvider, gql, InMemoryCache } from '@apollo/client';

async function write(filepath: string, source: string): Promise<void> {
    const options = await prettier.resolveConfig(filepath);
    if (!options) {
        throw new Error(`Can't resolve Prettier options for ${filepath}`);
    }
    await fs.promises.writeFile(
        filepath,
        prettier.format(source, {
            ...options,
            filepath,
        }),
        'utf8',
    );
}

async function mapGlobFiles<T>(pattern: string, map: (source: string) => T[]): Promise<T[]> {
    const fnames = await promisify(glob)(pattern);
    const resultSets = await Promise.all(fnames.map(async (fname) => map(await fs.promises.readFile(fname, 'utf8'))));
    return resultSets.flat();
}

/////////////////

async function renderIndex() {
    const client = new ApolloClient({
        ssrMode: true,
        cache: new InMemoryCache(),
    });
    client.cache.writeQuery<GetBot>({
        query: gql`
            query GetBot {
                bot {
                    id
                    name
                    avatar
                }
            }
        `,
        data: {
            bot: {
                __typename: 'User',
                id: '(BOT_ID)',
                name: '(BOT_NAME)',
                avatar: '(BOT_AVATAR)',
            },
        },
    });

    const body = render(
        <ApolloProvider client={client}>
            <App />
        </ApolloProvider>,
        {},
        {
            pretty: true,
        },
    );

    const html = await fs.promises.readFile('./src/index.template.html', 'utf8');
    await write('./src/index.html', html.replace(/<body>[\s\S]*<\/body>/, `<body>${body}</body>`));
}

async function renderUikitScript() {
    const icons = await mapGlobFiles('./src/**/*.tsx', (source) =>
        Array.from(source.matchAll(/uk-icon="([^"]+)"/g), (match) => match[1]),
    );
    await write(
        './src/uikit.ts',
        `
            import UIkit from 'uikit';
            UIkit.icon.add({
                ${Array.from(new Set(icons))
                    .map((icon) => `'${icon}': require('bundle-text:uikit/src/images/icons/${icon}.svg'),`)
                    .join('\n')}
            });
        `,
    );
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

    await write('./src/uikit.less', usedComponents.concat(usedThemes).join('\n'));
}

/////////////////

Promise.all([renderIndex(), renderUikitScript(), renderUikitStyle()]).catch((err) => {
    console.error(err);
    process.exit(1);
});
