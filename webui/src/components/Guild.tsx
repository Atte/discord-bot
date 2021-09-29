import React from 'react';
import Async from 'react-async';
import { Alert } from 'react-bootstrap';
import { GuildData } from './Guilds';

interface Role {
    id: string;
    guild_id: string;
    color: number;
    hoist: boolean;
    managed: boolean;
    mentionable: boolean;
    name: string;
    permissions: number | string;
    position: number;
    tags: {
        bot_id?: string,
        integration_id?: string,
        premium_subscriber: boolean,
    };
}

interface GuildRanks {
    current: Role[];
    available: Role[];
}

async function fetchGuildRanks({ id }: { id: string }, { signal }: { signal: AbortSignal }): Promise<GuildRanks> {
    const response = await fetch(`me/guilds/${id}/ranks`, { signal });
    if (!response.ok) {
        throw new Error(response.statusText);
    }
    return response.json();
}

export default function Guild(props: { guild: GuildData }): JSX.Element {
    return <Async promiseFn={fetchGuildRanks as any} id={props.guild.id}>
        <Async.Pending>Loading guilds...</Async.Pending>
        <Async.Rejected>
            <Alert variant="danger">
                <Alert.Heading>Rank load error</Alert.Heading>
                <p>{(error: Error) => error.message}</p>
            </Alert>
        </Async.Rejected>
        <Async.Fulfilled>{(data: GuildRanks) => <>
            Current:
            <ul>
                {data.current.map(role => <li>{role.name}</li>)}
            </ul>
            Available:
            <ul>
                {data.available.map(role => <li>{role.name}</li>)}
            </ul>
        </>}</Async.Fulfilled>
    </Async>;
}
