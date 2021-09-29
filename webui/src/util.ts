export function unreachable(x: never): never {
    throw new Error(`Unreachable reached: ${x}`);
}
