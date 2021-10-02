import { render } from 'preact';
import UIkit from 'uikit';
import Icons from 'uikit/dist/js/uikit-icons';
import App from './components/App';

(UIkit.use as Function)(Icons);
render(<App />, document.body);
