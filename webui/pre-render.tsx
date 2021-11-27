require('module-alias/register');

import * as fs from 'fs';
import App from './src/components/App';
import { render } from 'preact-render-to-string';
import { GetBot } from './src/components/__generated__/GetBot';
import { ApolloClient, ApolloProvider, gql, InMemoryCache } from '@apollo/client';
import { write } from './util';

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

renderIndex().catch((err) => {
    console.error(err);
    process.exit(1);
});
