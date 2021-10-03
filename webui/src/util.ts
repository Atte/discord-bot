import { useState, useEffect } from 'preact/hooks';

export function unreachable(x: never): never {
    throw new Error(`Unreachable reached: ${x}`);
}

export function sortBy<T extends { [key in K]: string }, K extends keyof T>(key: K): (a: T, b: T) => number {
    return (a, b) => a[key].toLowerCase().localeCompare(b[key].toLowerCase(), 'en');
}

export function useFetch<T>(input: RequestInfo, init?: RequestInit) {
    const [response, setResponse] = useState<T>();
    const [error, setError] = useState<Error>();

    useEffect(() => {
        let aborted = false;

        const abortController = new AbortController();
        abortController.signal.addEventListener('abort', () => {
            aborted = true;
        });
        
        async function inner() {
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
                    throw new Error(res.status.toString())
                }

                const data = await res.json();
                if (!aborted) {
                    setResponse(data);
                }
            } catch (err) {
                if (!aborted) {
                    setError(err as Error);
                }
            }
        }
        inner();

        return function() {
            aborted = true;
            abortController.abort();
        };
    }, [input, init]);

    return [response, error, setResponse, setError] as const;
}
