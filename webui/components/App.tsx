import React from 'react';
import Async from 'react-async';
import { Alert, Container, Button } from 'react-bootstrap';
import Guilds from './Guilds';

export interface CurrentUserData {
    id: number,
    avatar?: string,
    bot: boolean,
    discriminator: number,
    email?: string,
    mfa_enabled: boolean,
    username: string,
    verified?: boolean,
    public_flags?: { bits: number },
}

async function fetchCurrentUser({}, { signal }): Promise<CurrentUserData | null> {
    const response = await fetch('me/user', { signal });
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
        <Async promiseFn={fetchCurrentUser}>
            <Async.Pending>Loading session...</Async.Pending>
            <Async.Rejected>
                <Alert variant="danger">
                    <Alert.Heading>Login error</Alert.Heading>
                    <p>{error => error.message}</p>
                </Alert>
            </Async.Rejected>
            <Async.Fulfilled>{(data: CurrentUserData | null) =>
                data
                ? <><p>{data.username}#{data.discriminator}</p><Guilds /></>
                : 'Redirecting to login...'
            }</Async.Fulfilled>
        </Async>
    </Container>;
}
