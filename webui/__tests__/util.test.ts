import { sortByProp } from '../src/util';

describe('sortBy', () => {
    test('basic strings', () => {
        const items = [{ foo: 'bbb' }, { foo: 'Ccc' }, { foo: 'C' }, { foo: 'dd' }, { foo: 'AAA' }];
        expect(items.sort(sortByProp('foo'))).toEqual([
            { foo: 'AAA' },
            { foo: 'bbb' },
            { foo: 'C' },
            { foo: 'Ccc' },
            { foo: 'dd' },
        ]);
    });
});