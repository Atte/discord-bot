import React, { useState } from 'react';
import Async, { IfFulfilled, IfPending, IfRejected, useFetch } from 'react-async';
import { Alert, Form } from 'react-bootstrap';
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

export default function GuildRanks(props: { guild: GuildData }): JSX.Element {
    const state = useFetch<GuildRanks>(`me/guilds/${props.guild.id}/ranks`, {
        headers: { accept: 'application/json' },
    });
    const [changing, setChanging] = useState(false);

    async function setRole(role: Role, on: boolean): Promise<void> {
        setChanging(true);
        try {
            const response = await fetch(`me/guilds/${role.guild_id}/ranks/${role.id}`, { method: on ? 'POST' : 'DELETE' });
            if (!response.ok) {
                throw new Error(response.statusText);
            }
            state.setData(await response.json());
        } catch (err) {
            console.error(err);
            state.reload(); // something broke, reload to ensure state is valid
        } finally {
            setChanging(false);
        }
    }

    return <>
        <h3>{props.guild.name}</h3>
        <IfPending state={state}>Loading ranks...</IfPending>
        <IfRejected state={state}>
            <Alert variant="danger">
                <Alert.Heading>Rank load error</Alert.Heading>
                <p>{(error: Error) => error.message}</p>
            </Alert>
        </IfRejected>
        <IfFulfilled state={state}>{(data: GuildRanks) => <Form>
                {data && data.current.concat(data.available).sort((a, b) => a.name.localeCompare(b.name)).map(role =>
                    <Form.Check
                        type="checkbox"
                        id={role.id}
                        label={role.name}
                        disabled={changing}
                        checked={data.current.includes(role)}
                        onChange={() => setRole(role, !data.current.includes(role))}
                    />
                )}
            </Form>
        }</IfFulfilled>
    </>;
}
