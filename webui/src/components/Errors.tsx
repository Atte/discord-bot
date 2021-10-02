export default function Errors(props: { errors: (Error | undefined)[] }) {
    return <>
        {props.errors.filter(err => err).map(err => <div class="uk-alert-danger" uk-alert>{err!.toString()}</div>)}
    </>
}