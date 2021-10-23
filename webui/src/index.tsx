if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { render, hydrate } from 'preact';
import { ApolloClient, InMemoryCache, ApolloProvider, gql } from '@apollo/client';
import App from './components/App';
import { GetBot, GetBot_bot } from './__generated__/GetBot';

const client = new ApolloClient({
    uri: '/api/graphql',
    cache: new InMemoryCache(),
});

if (process.env.NODE_ENV === 'development') {
    while (document.body.firstChild) {
        document.body.firstChild.remove();
    }
    client
        .query<GetBot>({
            query: gql`
                query GetBot {
                    bot {
                        id
                        name
                        discriminator
                        avatar
                    }
                }
            `,
        })
        .then(async (response) => {
            render(
                <ApolloProvider client={client}>
                    <App bot={response.data.bot} />
                </ApolloProvider>,
                document.body,
            );
        })
        .catch((err) => {
            console.error(err);
        });
} else {
    const botData = document.head.querySelector<HTMLScriptElement>('script[type="application/x-bot-user+json"]');
    const bot: GetBot_bot = JSON.parse(botData?.textContent!);
    hydrate(
        <ApolloProvider client={client}>
            <App bot={bot} />
        </ApolloProvider>,
        document.body,
    );
}
