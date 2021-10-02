import { route } from 'preact-router';
import { useEffect } from 'preact/hooks';

export default function Redirect({ to }: { to: string }) {
  useEffect(() => {
    route(to, true);
  }, [to]);
  return null;
}
