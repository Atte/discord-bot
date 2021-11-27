if (process.env.NODE_ENV === 'development') {
    require('preact/debug');
}

import { render, hydrate } from 'preact';
import { ApolloClient, InMemoryCache, ApolloProvider } from '@apollo/client';
import App from './components/App';

const client = new ApolloClient({
    uri: 'api/graphql',
    cache: new InMemoryCache(),
});

if (process.env.NODE_ENV !== 'development') {
    hydrate(
        <ApolloProvider client={client}>
            <App />
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
