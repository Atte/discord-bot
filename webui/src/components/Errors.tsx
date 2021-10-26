import { ComponentChildren } from 'preact';
import { memo } from 'preact/compat';

export default memo(Errors);
function Errors({ errors, children }: { errors: (Error | undefined)[]; children?: ComponentChildren }) {
    return (
        <>
            {errors
                .filter((err) => err)
                .map((err, index, errors) => (
                    <div key={index} class="uk-alert-danger uk-animation-slide-top-small uk-alert" uk-alert>
                        {err!.toString()}
                        {index === errors.length - 1 && children}
                    </div>
                ))}
        </>
    );
}
