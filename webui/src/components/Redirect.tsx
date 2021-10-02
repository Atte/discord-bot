import { route } from 'preact-router';
import { useEffect } from 'preact/hooks';

export default function Redirect(props: { to: string }) {
  useEffect(() => {
    route(props.to, true);
  }, [props.to]);
  return null;
}
