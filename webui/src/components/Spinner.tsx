import { useMediaQuery } from '../util';

export default function Spinner(props: { class?: string; ratio?: number }) {
    const reducedMotion = useMediaQuery('(prefers-reduced-motion)');
    return (
        <div class={props.class}>
            {reducedMotion ? <>Loading&hellip;</> : <div uk-spinner={`ratio: ${props.ratio ?? 1}`} />}
        </div>
    );
}
