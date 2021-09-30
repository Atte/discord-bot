import React from 'react';
import Async from 'react-async';
import { Alert } from 'react-bootstrap';
import GuildRanks from './GuildRanks';

export interface GuildData {
    id: string;
    icon?: string;
    name: string;
    owner: boolean;
    permissions: number | string;
}

async function fetchGuilds({}, { signal }: { signal: AbortSignal }): Promise<GuildData[]> {
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
                <p>{(error: Error) => error.message}</p>
            </Alert>
        </Async.Rejected>
        <Async.Fulfilled>{(data: GuildData[]) => data.map(guild => <GuildRanks guild={guild} />)}</Async.Fulfilled>
    </Async>;
}
