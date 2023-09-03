// https://github.com/babel/babel/issues/12522
Inferno.render( /*#__PURE__*/_jsx("p", {
  children: "Hello, World!"
}), document.getElementById('root')); // Imports are hoisted, so this is still ok

import 'app-polyfill/ie11';
import 'app-polyfill/stable';
import Inferno from 'react-dom';
import { jsx as _jsx } from "react/jsx-runtime";
