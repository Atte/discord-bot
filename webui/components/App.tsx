import React from 'react';
import { useAsync, IfPending, IfFulfilled, IfRejected } from 'react-async';
import { Alert, Container } from 'react-bootstrap';

export interface CurrentUser {
    id: number,
    avatar?: string,
    bot: boolean,
    discriminator: number,
    email?: string,
    mfa_enabled: boolean,
    name: string,
    verified?: boolean,
    public_flags?: { bits: number },
}

async function loadUser({}, { signal }): Promise<CurrentUser | null> {
    const response = await fetch('auth/user', { signal });
    if (response.status === 404) {
        return null;
    }
    if (!response.ok) {
        throw new Error(response.statusText);
    }
    return response.json();
}

export default function App(): JSX.Element {
    const state = useAsync({ promiseFn: loadUser });
    if (state.isSettled) {
        //window.location.href = 'auth/redirect';
        return null;
    }
    return <Container>
        <IfPending state={state}>Loading user information&hellip;</IfPending>
        <IfRejected state={state}><Alert variant="danger">{error => error.message}</Alert></IfRejected>
        <IfFulfilled state={state}>{data => JSON.stringify(data)}</IfFulfilled>
    </Container>;
}
