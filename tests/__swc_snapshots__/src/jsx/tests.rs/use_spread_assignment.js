/*#__PURE__*/ import { createComponentVNode, normalizeProps } from "inferno";
normalizeProps(createComponentVNode(2, Component, {
    y: 2,
    ...x,
    z: true
}));
