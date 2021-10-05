import { useState, useEffect } from 'preact/hooks';

export function unreachable(x: never): never {
    throw new Error(`Unreachable reached: ${x}`);
}

export function sortByKey<T>(key: (x: T) => number | string): (a: T, b: T) => number {
    return (a, b) => {
        const keyA = key(a);
        const keyB = key(b);
        return typeof keyA === 'number' && typeof keyB === 'number'
            ? keyA - keyB
            : keyA.toString().toLowerCase().localeCompare(keyB.toString().toLowerCase(), 'en');
    };
}

export class ResponseStatusError extends Error {
    constructor(message: string, public readonly response: Response) {
        super(message);
    }
}

export function useFetch<T>(input: RequestInfo, init?: RequestInit) {
    const [response, setResponse] = useState<T>();
    const [error, setError] = useState<ResponseStatusError | Error>();

    useEffect(() => {
        let aborted = false;

        const abortController = new AbortController();
        abortController.signal.addEventListener('abort', () => {
            aborted = true;
        });

        (async function () {
            try {
                const res = await fetch(input, {
                    headers: {
                        accept: 'application/json',
                        ...init?.headers,
                    },
                    ...init,
                    signal: abortController.signal,
                });
                if (!res.ok) {
                    throw new ResponseStatusError(`${res.status} ${res.statusText}`, res);
                }

                const data = await res.json();
                if (!aborted) {
                    setResponse(data);
                }
            } catch (err) {
                if (!aborted) {
                    setError(err instanceof Error ? err : new Error(`${err}`));
                }
            }
        })();

        return function () {
            aborted = true;
            abortController.abort();
        };
    }, [input, init]);

    return [response, error, setResponse, setError] as const;
}

export function useMediaQuery(query: string, defaultValue = false) {
    const [value, setValue] = useState(() => globalThis.matchMedia?.(query).matches ?? defaultValue);
    useEffect(() => {
        function onChange(event: MediaQueryListEvent) {
            setValue(event.matches);
        }
        const matchMedia = globalThis.matchMedia?.(query);
        matchMedia?.addEventListener('change', onChange);
        return () => {
            matchMedia?.removeEventListener('change', onChange);
        };
    }, [query]);
    return value;
}
