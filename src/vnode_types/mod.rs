use crate::inferno_flags::VNodeFlags;

pub fn parse_vnode_flag(tag: &str) -> u16 {
    match tag {
        "input" => VNodeFlags::InputElement as u16,
        "textarea" => VNodeFlags::TextareaElement as u16,
        "select" => VNodeFlags::SelectElement as u16,
        // SVG ELEMENTS
        "altGlyph" => VNodeFlags::SvgElement as u16,
        "altGlyphDef" => VNodeFlags::SvgElement as u16,
        "altGlyphItem" => VNodeFlags::SvgElement as u16,
        "animate" => VNodeFlags::SvgElement as u16,
        "animateColor" => VNodeFlags::SvgElement as u16,
        "animateMotion" => VNodeFlags::SvgElement as u16,
        "animateTransform" => VNodeFlags::SvgElement as u16,
        "circle" => VNodeFlags::SvgElement as u16,
        "clipPath" => VNodeFlags::SvgElement as u16,
        "color-profile" => VNodeFlags::SvgElement as u16,
        "cursor" => VNodeFlags::SvgElement as u16,
        "defs" => VNodeFlags::SvgElement as u16,
        "desc" => VNodeFlags::SvgElement as u16,
        "discard" => VNodeFlags::SvgElement as u16,
        "ellipse" => VNodeFlags::SvgElement as u16,
        "feBlend" => VNodeFlags::SvgElement as u16,
        "feColorMatrix" => VNodeFlags::SvgElement as u16,
        "feComponentTransfer" => VNodeFlags::SvgElement as u16,
        "feComposite" => VNodeFlags::SvgElement as u16,
        "feConvolveMatrix" => VNodeFlags::SvgElement as u16,
        "feDiffuseLighting" => VNodeFlags::SvgElement as u16,
        "feDisplacementMap" => VNodeFlags::SvgElement as u16,
        "feDistantLight" => VNodeFlags::SvgElement as u16,
        "feDropShadow" => VNodeFlags::SvgElement as u16,
        "feFlood" => VNodeFlags::SvgElement as u16,
        "feFuncA" => VNodeFlags::SvgElement as u16,
        "feFuncB" => VNodeFlags::SvgElement as u16,
        "feFuncG" => VNodeFlags::SvgElement as u16,
        "feFuncR" => VNodeFlags::SvgElement as u16,
        "feGaussianBlur" => VNodeFlags::SvgElement as u16,
        "feImage" => VNodeFlags::SvgElement as u16,
        "feMerge" => VNodeFlags::SvgElement as u16,
        "feMergeNode" => VNodeFlags::SvgElement as u16,
        "feMorphology" => VNodeFlags::SvgElement as u16,
        "feOffset" => VNodeFlags::SvgElement as u16,
        "fePointLight" => VNodeFlags::SvgElement as u16,
        "feSpecularLighting" => VNodeFlags::SvgElement as u16,
        "feSpotLight" => VNodeFlags::SvgElement as u16,
        "feTile" => VNodeFlags::SvgElement as u16,
        "feTurbulence" => VNodeFlags::SvgElement as u16,
        "filter" => VNodeFlags::SvgElement as u16,
        "font-face" => VNodeFlags::SvgElement as u16,
        "font-face-format" => VNodeFlags::SvgElement as u16,
        "font-face-name" => VNodeFlags::SvgElement as u16,
        "font-face-src" => VNodeFlags::SvgElement as u16,
        "font-face-uri" => VNodeFlags::SvgElement as u16,
        "foreignObject" => VNodeFlags::SvgElement as u16,
        "g" => VNodeFlags::SvgElement as u16,
        "glyph" => VNodeFlags::SvgElement as u16,
        "glyphRef" => VNodeFlags::SvgElement as u16,
        "hkern" => VNodeFlags::SvgElement as u16,
        "line" => VNodeFlags::SvgElement as u16,
        "linearGradient" => VNodeFlags::SvgElement as u16,
        "marker" => VNodeFlags::SvgElement as u16,
        "mask" => VNodeFlags::SvgElement as u16,
        "metadata" => VNodeFlags::SvgElement as u16,
        "missing-glyph" => VNodeFlags::SvgElement as u16,
        "mpath" => VNodeFlags::SvgElement as u16,
        "path" => VNodeFlags::SvgElement as u16,
        "pattern" => VNodeFlags::SvgElement as u16,
        "polygon" => VNodeFlags::SvgElement as u16,
        "polyline" => VNodeFlags::SvgElement as u16,
        "radialGradient" => VNodeFlags::SvgElement as u16,
        "rect" => VNodeFlags::SvgElement as u16,
        "set" => VNodeFlags::SvgElement as u16,
        "stop" => VNodeFlags::SvgElement as u16,
        "svg" => VNodeFlags::SvgElement as u16,
        "switch" => VNodeFlags::SvgElement as u16,
        "symbol" => VNodeFlags::SvgElement as u16,
        "text" => VNodeFlags::SvgElement as u16,
        "textPath" => VNodeFlags::SvgElement as u16,
        "tref" => VNodeFlags::SvgElement as u16,
        "tspan" => VNodeFlags::SvgElement as u16,
        "unknown" => VNodeFlags::SvgElement as u16,
        "use" => VNodeFlags::SvgElement as u16,
        "view" => VNodeFlags::SvgElement as u16,
        "vkern" => VNodeFlags::SvgElement as u16,
        "hatch" => VNodeFlags::SvgElement as u16,
        "hatchpath" => VNodeFlags::SvgElement as u16,
        "mesh" => VNodeFlags::SvgElement as u16,
        "meshgradient" => VNodeFlags::SvgElement as u16,
        "meshpatch" => VNodeFlags::SvgElement as u16,
        "meshrow" => VNodeFlags::SvgElement as u16,
        "solidcolor" => VNodeFlags::SvgElement as u16,
        _ => VNodeFlags::HtmlElement as u16,
    }
}

pub fn convert_svg_attrs(sym: &str) -> &str {
    match sym {
        "accentHeight" => "accent-height",
        "alignmentBaseline" => "alignment-baseline",
        "arabicForm" => "arabic-form",
        "baselineShift" => "baseline-shift",
        "capHeight" => "cap-height",
        "clipPath" => "clip-path",
        "clipRule" => "clip-rule",
        "colorInterpolation" => "color-interpolation",
        "colorInterpolationFilters" => "color-interpolation-filters",
        "colorProfile" => "color-profile",
        "colorRendering" => "color-rendering",
        "dominantBaseline" => "dominant-baseline",
        "enableBackground" => "enable-background",
        "fillOpacity" => "fill-opacity",
        "fillRule" => "fill-rule",
        "floodColor" => "flood-color",
        "floodOpacity" => "flood-opacity",
        "fontFamily" => "font-family",
        "fontSize" => "font-size",
        "fontSizeAdjust" => "font-size-adjust",
        "fontStretch" => "font-stretch",
        "fontStyle" => "font-style",
        "fontVariant" => "font-variant",
        "fontWeight" => "font-weight",
        "glyphName" => "glyph-name",
        "glyphOrientationHorizontal" => "glyph-orientation-horizontal",
        "glyphOrientationVertical" => "glyph-orientation-vertical",
        "horizAdvX" => "horiz-adv-x",
        "horizOriginX" => "horiz-origin-x",
        "imageRendering" => "image-rendering",
        "letterSpacing" => "letter-spacing",
        "lightingColor" => "lighting-color",
        "markerEnd" => "marker-end",
        "markerMid" => "marker-mid",
        "markerStart" => "marker-start",
        "markerHeight" => "markerHeight",
        "overlinePosition" => "overline-position",
        "overlineThickness" => "overline-thickness",
        "paintOrder" => "paint-order",
        "panose1" => "panose-1",
        "pointerEvents" => "pointer-events",
        "renderingIntent" => "rendering-intent",
        "shapeRendering" => "shape-rendering",
        "stopColor" => "stop-color",
        "stopOpacity" => "stop-opacity",
        "strikethroughPosition" => "strikethrough-position",
        "strikethroughThickness" => "strikethrough-thickness",
        "strokeDasharray" => "stroke-dasharray",
        "strokeDashoffset" => "stroke-dashoffset",
        "strokeLinecap" => "stroke-linecap",
        "strokeLinejoin" => "stroke-linejoin",
        "strokeMiterlimit" => "stroke-miterlimit",
        "strokeOpacity" => "stroke-opacity",
        "strokeWidth" => "stroke-width",
        "textDecoration" => "text-decoration",
        "textRendering" => "text-rendering",
        "underlinePosition" => "underline-position",
        "underlineThickness" => "underline-thickness",
        "unicodeBidi" => "unicode-bidi",
        "unicodeRange" => "unicode-range",
        "unitsPerEm" => "units-per-em",
        "vAlphabetic" => "v-alphabetic",
        "vHanging" => "v-hanging",
        "vIdeographic" => "v-ideographic",
        "vMathematical" => "v-mathematical",
        "vectorEffect" => "vector-effect",
        "vertAdvY" => "vert-adv-y",
        "vertOriginX" => "vert-origin-x",
        "vertOriginY" => "vert-origin-y",
        "wordSpacing" => "word-spacing",
        "writingMode" => "writing-mode",
        "xHeight" => "x-height",
        "xlinkActuate" => "xlink:actuate",
        "xlinkArcrole" => "xlink:arcrole",
        "xlinkHref" => "xlink:href",
        "xlinkRole" => "xlink:role",
        "xlinkShow" => "xlink:show",
        "xlinkTitle" => "xlink:title",
        "xlinkType" => "xlink:type",
        "xmlBase" => "xml:base",
        "xmlnsXlink" => "xmlns:xlink",
        "xmlLang" => "xml:lang",
        "xmlSpace" => "xml:space",
        _ => sym,
    }
}
