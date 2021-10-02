export default function Errors({ errors }: { errors: (Error | undefined)[] }) {
    return <>
        {errors.filter(err => err).map(err => <div class="uk-alert-danger uk-animation-slide-top-small" uk-alert>{err!.toString()}</div>)}
    </>
}