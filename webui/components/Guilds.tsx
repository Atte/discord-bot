import React from 'react';
import Async from 'react-async';
import { Alert } from 'react-bootstrap';
import Guild from './Guild';

export interface GuildData {
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

async function fetchGuilds({}, { signal }): Promise<GuildData[]> {
    const response = await fetch('me/guilds', { signal });
    if (!response.ok) {
        throw new Error(response.statusText);
    }
    return response.json();
}

export default function Guilds(): JSX.Element {
    return <Async promiseFn={fetchGuilds}>
        <Async.Pending>Loading guilds...</Async.Pending>
        <Async.Rejected>
            <Alert variant="danger">
                <Alert.Heading>Guild load error</Alert.Heading>
                <p>{error => error.message}</p>
            </Alert>
        </Async.Rejected>
        <Async.Fulfilled>{(data: GuildData[]) => data.map(guild => <Guild guild={guild} />)}</Async.Fulfilled>
    </Async>;
}
