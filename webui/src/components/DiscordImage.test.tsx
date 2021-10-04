import { render as originalRender, screen } from '@testing-library/preact';
import { ComponentChild } from 'preact';
import DiscordImage from './DiscordImage';

function render(child: ComponentChild): HTMLImageElement {
    originalRender(child);
    return screen.getByRole('img') as HTMLImageElement;
}

function mockReducedMotion(matches: boolean): void {
    // https://jestjs.io/docs/manual-mocks#mocking-methods-which-are-not-implemented-in-jsdom
    Object.defineProperty(window, 'matchMedia', {
        writable: true,
        value: jest.fn().mockImplementation((query) => ({
            matches,
            media: query,
            onchange: null,
            addListener: jest.fn(), // deprecated
            removeListener: jest.fn(), // deprecated
            addEventListener: jest.fn(),
            removeEventListener: jest.fn(),
            dispatchEvent: jest.fn(),
        })),
    });
}

describe('no reduced motion', () => {
    beforeAll(() => {
        mockReducedMotion(false);
    });

    test('avatar', async () => {
        const img = render(<DiscordImage type="avatar" user_id="user" user_avatar="a_avatar" size={16} />);
        expect(img.src).toBe('https://cdn.discordapp.com/avatars/user/a_avatar.webp?size=16');
    });

    test('animated circular icon', async () => {
        const img = render(<DiscordImage type="icon" guild_id="guild" guild_icon="a_icon" size={64} animated circle />);
        expect(img.src).toBe('https://cdn.discordapp.com/icons/guild/a_icon.gif?size=64');
        expect(img.style.borderRadius).not.toHaveLength(0);
    });
});

describe('reduced motion', () => {
    beforeAll(() => {
        mockReducedMotion(true);
    });

    test('avatar', async () => {
        const img = render(<DiscordImage type="avatar" user_id="user" user_avatar="a_avatar" size={16} />);
        expect(img.src).toBe('https://cdn.discordapp.com/avatars/user/a_avatar.webp?size=16');
    });

    test('animated circular icon', async () => {
        const img = render(<DiscordImage type="icon" guild_id="guild" guild_icon="a_icon" size={64} animated circle />);
        expect(img.src).toBe('https://cdn.discordapp.com/icons/guild/a_icon.webp?size=64');
        expect(img.style.borderRadius).not.toHaveLength(0);
    });
});
