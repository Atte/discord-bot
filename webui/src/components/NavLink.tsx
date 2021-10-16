import { ComponentChildren } from 'preact';
import Match from 'preact-router/match';

export function NavLink({ path, children }: { path: string; children: ComponentChildren }) {
    return (
        <Match path={`${path}/:rest*`}>
            {({ matches }: { matches: boolean }) => (
                <li class={matches ? 'uk-active' : ''}>
                    <a href={path}>{children}</a>
                </li>
            )}
        </Match>
    );
}
