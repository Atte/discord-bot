export default function Spinner(props: { class?: string; ratio?: number }) {
    const animate = !window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    return (
        <div class={props.class}>
            {animate ? <div uk-spinner={`ratio: ${props.ratio ?? 1}`} /> : <>Loading&hellip;</>}
        </div>
    );
}
