import { useState, useEffect, Inputs } from 'preact/hooks';

export function unreachable(x: never): never {
    throw new Error(`Unreachable reached: ${x}`);
}

export function useFetch<T>(input: RequestInfo, init?: RequestInit, dependencies?: Inputs) {
    const [response, setResponse] = useState<T>();
    const [error, setError] = useState<Error>();

    let abortController: AbortController;
    useEffect(() => {
        let aborted = false;
        abortController = new AbortController();
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
    }, [input, init, ...(dependencies ?? [])]);

    return [response, error, setResponse, setError] as const;
}
