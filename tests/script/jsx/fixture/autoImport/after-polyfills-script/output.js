// https://github.com/babel/babel/issues/12522
const { createVNode } = require("inferno");
require("app-polyfill/ie11");
require("app-polyfill/stable");
const Inferno = require("inferno");
Inferno.render(/*#__PURE__*/ createVNode(1, "p", null, "Hello, World!", 16), document.getElementById("root"));
