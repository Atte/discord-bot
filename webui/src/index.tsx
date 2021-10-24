if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { render, hydrate } from 'preact';
import { ApolloClient, InMemoryCache, ApolloProvider } from '@apollo/client';
import App from './components/App';
import { GetBot_bot } from './components/__generated__/GetBot';

const client = new ApolloClient({
    uri: '/api/graphql',
    cache: new InMemoryCache(),
});

const inlineSource = document.head.querySelector<HTMLScriptElement>('script[type="application/x-bot-user+json"]');
const inlineBot: GetBot_bot = inlineSource?.textContent && JSON.parse(inlineSource.textContent);

if (inlineBot && process.env.NODE_ENV !== 'development') {
    hydrate(
        <ApolloProvider client={client}>
            <App bot={inlineBot} />
        </ApolloProvider>,
        document.body,
    );
} else {
    while (document.body.firstChild) {
        document.body.firstChild.remove();
    }
    render(
        <ApolloProvider client={client}>
            <App />
        </ApolloProvider>,
        document.body,
    );
}
