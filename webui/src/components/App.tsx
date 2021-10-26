import { useEffect, useErrorBoundary } from 'preact/hooks';
import Errors from './Errors';
import NavbarBot from './NavbarBot';
import NavbarUser from './NavbarUser';
import Guilds from './Guilds';
import { memo } from 'preact/compat';

export default memo(App);
function App() {
    const [childError] = useErrorBoundary();

    useEffect(() => {
        // UIkit only does some additional styling,
        // so only load it after first paint to seem faster
        import('../uikit');
    }, []);

    return (
        <div class="uk-container">
            <nav class="uk-navbar-container uk-flex-wrap uk-navbar" uk-navbar>
                <div class="uk-navbar-left">
                    <div class="uk-navbar-item uk-logo">
                        <NavbarBot />
                    </div>
                </div>
                <div class="uk-navbar-right">
                    <NavbarUser />
                </div>
            </nav>
            {childError?.message === 'unauthenticated' ? (
                <div class="uk-padding-small">
                    <form action="api/auth/redirect" method="POST">
                        <button class="uk-button uk-button-primary uk-animation-fade uk-animation-fast">
                            <span uk-icon="sign-in" /> Sign in with Discord
                        </button>
                    </form>
                </div>
            ) : (
                <Errors errors={[childError]}>
                    <form action="api/auth/clear" method="POST">
                        <button class="uk-button uk-button-primary">
                            <span uk-icon="refresh" /> Retry
                        </button>
                    </form>
                </Errors>
            )}
            <Guilds />
        </div>
    );
}
