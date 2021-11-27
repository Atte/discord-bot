import * as fs from 'fs';
import * as prettier from 'prettier';

export async function write(filepath: string, source: string): Promise<void> {
    const options = await prettier.resolveConfig(filepath);
    if (!options) {
        throw new Error(`Can't resolve Prettier options for ${filepath}`);
    }
    await fs.promises.writeFile(
        filepath,
        prettier.format(source, {
            ...options,
            filepath,
        }),
        'utf8',
    );
}
