import { createVNode, normalizeProps, createFragment } from "inferno";
var x = /*#__PURE__*/ createFragment([
  /*#__PURE__*/ createVNode(1, "div", null, [
    /*#__PURE__*/ createVNode(1, "div", null, null, 1, null, "1"),
    /*#__PURE__*/ createVNode(1, "div", null, null, 1, {
      meow: "wolf"
    }, "2"),
    /*#__PURE__*/ createVNode(1, "div", null, null, 1, null, "3"),
    /*#__PURE__*/ normalizeProps(createVNode(1, "div", null, null, 1, {
      ...props
    }, "4"))
  ], 8)
], 4);
