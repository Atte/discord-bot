import React from 'react';
import Async from 'react-async';
import { Alert, Container, Button } from 'react-bootstrap';

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
        window.location.href = 'auth/redirect';
        return null;
    }
    if (!response.ok) {
        throw new Error(response.statusText);
    }
    return response.json();
}

function clearStorage(): void {
    window.location.href = 'auth/clear';
}

export default function App(): JSX.Element {
    return <Container>
        <Button variant="primary" onClick={clearStorage}>Log out</Button>
        <Async promiseFn={loadUser}>
            <Async.Pending>Loading session...</Async.Pending>
            <Async.Rejected>
                <Alert variant="danger">
                    <Alert.Heading>Login error</Alert.Heading>
                    <p>{error => error.message}</p>
                </Alert>
            </Async.Rejected>
            <Async.Fulfilled>{data => data ? JSON.stringify(data) : 'Redirecting to login...'}</Async.Fulfilled>
        </Async>
    </Container>;
}
