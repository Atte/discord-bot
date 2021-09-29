import React from 'react';
import Async from 'react-async';
import { Alert, Container, Button, Navbar, Nav } from 'react-bootstrap';
import { HashRouter as Router, Switch, Route, Link, Redirect } from 'react-router-dom';
import DiscordImage from './DiscordImage';
import Guilds from './Guilds';

export interface CurrentUserData {
    id: string;
    avatar?: string;
    bot: boolean;
    discriminator: number;
    email?: string;
    mfa_enabled: boolean;
    username: string;
    verified?: boolean;
    public_flags?: number;
}

async function fetchCurrentUser({}, { signal }: { signal: AbortSignal }): Promise<CurrentUserData | null> {
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

async function fetchBotUser({}, { signal }: { signal: AbortSignal }): Promise<CurrentUserData | null> {
    const response = await fetch('bot/user', { signal });
    if (!response.ok) {
        throw new Error(response.statusText);
    }
    const data: CurrentUserData | null = await response.json();
    if (data?.username) {
        document.title = data.username;
    }
    return data;
}

function clearStorage(): void {
    window.location.href = 'auth/clear';
}

export default function App(): JSX.Element {
    const logout = <Button variant="primary" onClick={clearStorage} className="ms-3 me-3">Log out</Button>;
    return <Container>
        <Async promiseFn={fetchCurrentUser}>
            <Async.Pending>Loading session... {logout}</Async.Pending>
            <Async.Rejected>
                <Alert variant="danger">
                    <Alert.Heading>Login error</Alert.Heading>
                    <p>{(error: Error) => error.message}</p>
                    {logout}
                </Alert>
            </Async.Rejected>
            <Async.Fulfilled>{(user: CurrentUserData | null) =>
                !user
                ? 'Redirecting to login...'
                : <Router>
                    <Navbar bg="dark" variant="dark">
                        <Async promiseFn={fetchBotUser}>
                            <Async.Fulfilled>{(bot: CurrentUserData) => <Navbar.Brand className="ms-3">{bot.username}</Navbar.Brand>}</Async.Fulfilled>
                        </Async>
                        <Nav className="me-auto">
                            <Nav.Link as={Link} to="/ranks">Ranks</Nav.Link>
                        </Nav>
                        {user.avatar && <DiscordImage type="avatar" user_id={user.id} user_avatar={user.avatar} size={32} />}
                        <Navbar.Text className="fw-bold">{user.username}#{user.discriminator}</Navbar.Text>
                        {logout}
                    </Navbar>
                    <Switch>
                        <Redirect exact from="/" to="/ranks" />
                        <Route path="/ranks">
                            <Guilds />
                        </Route>
                    </Switch>
                </Router>
            }</Async.Fulfilled>
        </Async>
    </Container>;
}
