import { route } from 'preact-router';
import { memo } from 'preact/compat';
import { useEffect } from 'preact/hooks';

export default memo(Redirect);
function Redirect({ to }: { to: string }) {
    useEffect(() => {
        route(to, true);
    }, [to]);
    return null;
}
