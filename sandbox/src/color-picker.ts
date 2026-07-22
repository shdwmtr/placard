import {
    EditorView,
    WidgetType,
    ViewPlugin,
    Decoration,
    type DecorationSet,
    type ViewUpdate,
} from "@codemirror/view";
import { NodeProp, type Tree } from "@lezer/common";
import { syntaxTree } from "@codemirror/language";
import type { Text, Range } from "@codemirror/state";
import { namedColors } from "./named-colors";

const ColorType = { rgb: "RGB", hex: "HEX", named: "NAMED", hsl: "HSL" } as const;
type ColorTypeValue = (typeof ColorType)[keyof typeof ColorType];

interface ColorData {
    colorType: ColorTypeValue;
    color: string;
    alpha: string;
}

interface PickerState {
    from: number;
    to: number;
    alpha: string;
    colorType: ColorTypeValue;
}

interface WidgetOptions extends PickerState {
    color: string;
}

const rgbCallExpRegex =
    /rgb(?:a)?\(\s*(\d{1,3}%?)\s*,?\s*(\d{1,3}%?)\s*,?\s*(\d{1,3}%?)\s*([,/]\s*0?\.?\d+%?)?\)/;
const hslCallExpRegex =
    /hsl\(\s*(\d{1,3})\s*,\s*(\d{1,3})%\s*,\s*(\d{1,3})%\s*(,\s*0?\.\d+)?\)/;
const hexRegex = /(^|\b)(#[0-9a-f]{3,9})(\b|$)/i;

function parseCallExpression(callExp: string): ColorData | null {
    const fn = callExp.slice(0, 3);

    if (fn === "rgb") {
        const match = rgbCallExpRegex.exec(callExp);
        if (!match) return null;
        const [, r, g, b, a] = match;
        return { colorType: ColorType.rgb, color: rgbToHex(r, g, b), alpha: a || "" };
    }

    if (fn === "hsl") {
        const match = hslCallExpRegex.exec(callExp);
        if (!match) return null;
        const [, h, s, l, a] = match;
        return { colorType: ColorType.hsl, color: hslToHex(h, s, l), alpha: a || "" };
    }

    return null;
}

function parseColorLiteral(colorLiteral: string): ColorData | null {
    const match = hexRegex.exec(colorLiteral);
    if (!match) return null;
    const [color, alpha] = toFullHex(colorLiteral);
    return { colorType: ColorType.hex, color, alpha };
}

function parseNamedColor(colorName: string): ColorData | null {
    const color = namedColors.get(colorName);
    if (!color) return null;
    return { colorType: ColorType.named, color, alpha: "" };
}

function discoverColorsInCSS(
    tree: Tree,
    from: number,
    to: number,
    typeName: string,
    doc: Text,
): WidgetOptions | WidgetOptions[] | null {
    switch (typeName) {
        case "AttributeValue": {
            const innerTree = tree.resolveInner(from, 0).tree;
            if (!innerTree) return null;

            const overlayTree = innerTree.prop(NodeProp.mounted)?.tree;
            if (overlayTree?.type.name !== "Styles") return null;

            const ret: WidgetOptions[] = [];
            overlayTree.iterate({
                from: 0,
                to: overlayTree.length,
                enter: ({ type, from: overlayFrom, to: overlayTo }) => {
                    const maybe = discoverColorsInCSS(
                        tree,
                        from + 1 + overlayFrom,
                        from + 1 + overlayTo,
                        type.name,
                        doc,
                    );
                    if (maybe) {
                        if (Array.isArray(maybe)) throw new Error("unexpected nested overlays");
                        ret.push(maybe);
                    }
                },
            });
            return ret;
        }

        case "CallExpression": {
            const result = parseCallExpression(doc.sliceString(from, to));
            return result ? { ...result, from, to } : null;
        }

        case "ColorLiteral": {
            const result = parseColorLiteral(doc.sliceString(from, to));
            return result ? { ...result, from, to } : null;
        }

        case "ValueName": {
            const result = parseNamedColor(doc.sliceString(from, to));
            return result ? { ...result, from, to } : null;
        }

        default:
            return null;
    }
}

function toFullHex(color: string): [string, string] {
    if (color.length === 4) {
        return [`#${color[1].repeat(2)}${color[2].repeat(2)}${color[3].repeat(2)}`, ""];
    }
    if (color.length === 5) {
        return [`#${color[1].repeat(2)}${color[2].repeat(2)}${color[3].repeat(2)}`, color[4].repeat(2)];
    }
    if (color.length === 9) {
        return [`#${color.slice(1, -2)}`, color.slice(-2)];
    }
    return [color, ""];
}

function decimalToHex(decimal: number): string {
    const hex = decimal.toString(16);
    return hex.length === 1 ? "0" + hex : hex;
}

function rgbComponentToHex(component: string): string {
    const numericValue = component.endsWith("%")
        ? Math.round((Number(component.slice(0, -1)) / 100) * 255)
        : Number(component);
    return decimalToHex(numericValue);
}

function rgbToHex(r: string, g: string, b: string): string {
    return `#${rgbComponentToHex(r)}${rgbComponentToHex(g)}${rgbComponentToHex(b)}`;
}

function hexToRGBComponents(hex: string): [number, number, number] {
    return [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16)];
}

function clamp(num: number): number {
    if (num < 0) return num + 1;
    if (num > 1) return num - 1;
    return num;
}

function hueToRGB(temp1: number, temp2: number, tempHue: number): number {
    if (6 * tempHue < 1) return temp2 + (temp1 - temp2) * 6 * tempHue;
    if (2 * tempHue < 1) return temp1;
    if (3 * tempHue < 2) return temp2 + (temp1 - temp2) * (0.666 - tempHue) * 6;
    return temp2;
}

function hslToRGB(hue: number, saturation: number, luminance: number): [number, number, number] {
    if (saturation === 0) {
        const value = Math.round(luminance * 255);
        return [value, value, value];
    }

    const temp1 =
        luminance < 0.5
            ? luminance * (1.0 + saturation)
            : luminance + saturation - luminance * saturation;
    const temp2 = 2 * luminance - temp1;
    hue = hue / 360.0;

    const red = hueToRGB(temp1, temp2, clamp(hue + 0.333));
    const green = hueToRGB(temp1, temp2, hue);
    const blue = hueToRGB(temp1, temp2, clamp(hue - 0.333));
    return [Math.round(red * 255), Math.round(green * 255), Math.round(blue * 255)];
}

function hslToHex(h: string, s: string, l: string): string {
    const [r, g, b] = hslToRGB(Number(h), Number(s) / 100, Number(l) / 100);
    return `#${decimalToHex(r)}${decimalToHex(g)}${decimalToHex(b)}`;
}

function rgbToHSL(r: number, g: number, b: number): [number, number, number] {
    const redPercent = r / 255;
    const greenPercent = g / 255;
    const bluePercent = b / 255;
    const min = Math.min(redPercent, greenPercent, bluePercent);
    const max = Math.max(redPercent, greenPercent, bluePercent);
    const luminance = (max + min) / 2;

    if (max === min) return [0, 0, luminance];

    const saturation =
        luminance <= 0.5 ? (max - min) / (max + min) : (max - min) / (2.0 - max - min);

    let hue: number;
    if (max === redPercent) {
        hue = (greenPercent - bluePercent) / (max - min);
    } else if (max === greenPercent) {
        hue = 2.0 + (bluePercent - redPercent) / (max - min);
    } else {
        hue = 4.0 + (redPercent - greenPercent) / (max - min);
    }
    hue = Math.round(hue * 60);
    while (hue < 0) hue += 360;

    return [hue, saturation, luminance];
}

function hexToHsv(hex: string): { h: number; s: number; v: number } {
    const [r, g, b] = hexToRGBComponents(hex);
    const rf = r / 255;
    const gf = g / 255;
    const bf = b / 255;
    const max = Math.max(rf, gf, bf);
    const min = Math.min(rf, gf, bf);
    const d = max - min;

    let h = 0;
    if (d !== 0) {
        if (max === rf) h = ((gf - bf) / d) % 6;
        else if (max === gf) h = (bf - rf) / d + 2;
        else h = (rf - gf) / d + 4;
        h *= 60;
        if (h < 0) h += 360;
    }

    return { h, s: max === 0 ? 0 : d / max, v: max };
}

function hsvToHex(h: number, s: number, v: number): string {
    const c = v * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = v - c;
    let rf = 0;
    let gf = 0;
    let bf = 0;

    if (h < 60) { rf = c; gf = x; }
    else if (h < 120) { rf = x; gf = c; }
    else if (h < 180) { gf = c; bf = x; }
    else if (h < 240) { gf = x; bf = c; }
    else if (h < 300) { rf = x; bf = c; }
    else { rf = c; bf = x; }

    return `#${decimalToHex(Math.round((rf + m) * 255))}${decimalToHex(Math.round((gf + m) * 255))}${decimalToHex(Math.round((bf + m) * 255))}`;
}

function clamp01(n: number): number {
    return Math.min(1, Math.max(0, n));
}

interface ActiveFlyout {
    el: HTMLDivElement;
    anchor: HTMLElement;
    onOutsidePointerDown: (e: PointerEvent) => void;
    onKeyDown: (e: KeyboardEvent) => void;
}

let activeFlyout: ActiveFlyout | null = null;

function closeFlyout() {
    if (!activeFlyout) return;
    activeFlyout.el.remove();
    document.removeEventListener("pointerdown", activeFlyout.onOutsidePointerDown, true);
    document.removeEventListener("keydown", activeFlyout.onKeyDown, true);
    activeFlyout = null;
}

function openFlyout(view: EditorView, anchor: HTMLElement, state: PickerState, color: string) {
    if (activeFlyout && activeFlyout.anchor === anchor) {
        closeFlyout();
        return;
    }
    closeFlyout();

    let { h, s, v } = hexToHsv(color);
    let from = state.from;
    let to = state.to;
    const alpha = state.alpha;
    const colorType = state.colorType;

    const el = document.createElement("div");
    el.className = "cm-color-flyout";

    const svArea = document.createElement("div");
    svArea.className = "cm-color-flyout-sv";
    const svThumb = document.createElement("div");
    svThumb.className = "cm-color-flyout-thumb";
    svArea.appendChild(svThumb);

    const hueArea = document.createElement("div");
    hueArea.className = "cm-color-flyout-hue";
    const hueThumb = document.createElement("div");
    hueThumb.className = "cm-color-flyout-thumb";
    hueArea.appendChild(hueThumb);

    const hexRow = document.createElement("div");
    hexRow.className = "cm-color-flyout-hex-row";
    const preview = document.createElement("span");
    preview.className = "cm-color-flyout-preview";
    const hexInput = document.createElement("input");
    hexInput.type = "text";
    hexInput.className = "cm-color-flyout-hex";
    hexInput.spellcheck = false;
    hexRow.appendChild(preview);
    hexRow.appendChild(hexInput);

    el.appendChild(svArea);
    el.appendChild(hueArea);
    el.appendChild(hexRow);
    document.body.appendChild(el);

    function currentHex(): string {
        return hsvToHex(h, s, v);
    }

    function render() {
        const hex = currentHex();
        svArea.style.backgroundColor = `hsl(${h}, 100%, 50%)`;
        svThumb.style.left = s * 100 + "%";
        svThumb.style.top = (1 - v) * 100 + "%";
        hueThumb.style.left = (h / 360) * 100 + "%";
        preview.style.background = hex;
        hexInput.value = hex;
    }

    function commit() {
        const hex = currentHex();
        let converted = hex + alpha;

        if (colorType === ColorType.rgb) {
            converted = `rgb(${hexToRGBComponents(hex).join(", ")}${alpha})`;
        } else if (colorType === ColorType.named) {
            converted = hex;
            for (const [name, value] of namedColors.entries()) {
                if (value === hex) {
                    converted = name;
                    break;
                }
            }
        } else if (colorType === ColorType.hsl) {
            const [r, g, b] = hexToRGBComponents(hex);
            const [hh, ss, ll] = rgbToHSL(r, g, b);
            converted = `hsl(${hh}, ${Math.round(ss * 100)}%, ${Math.round(ll * 100)}%${alpha})`;
        }

        view.dispatch({ changes: { from, to, insert: converted } });
        to = from + converted.length;
    }

    function pointerDrag(area: HTMLElement, onMove: (e: PointerEvent) => void) {
        area.addEventListener("pointerdown", (e) => {
            e.preventDefault();
            area.setPointerCapture(e.pointerId);
            let scheduled = false;
            let lastEvent = e;

            const apply = () => {
                scheduled = false;
                onMove(lastEvent);
                render();
                commit();
            };
            const move = (ev: PointerEvent) => {
                lastEvent = ev;
                if (!scheduled) {
                    scheduled = true;
                    requestAnimationFrame(apply);
                }
            };
            const up = () => {
                area.removeEventListener("pointermove", move);
                area.removeEventListener("pointerup", up);
            };

            move(e);
            area.addEventListener("pointermove", move);
            area.addEventListener("pointerup", up);
        });
    }

    pointerDrag(svArea, (e) => {
        const rect = svArea.getBoundingClientRect();
        s = clamp01((e.clientX - rect.left) / rect.width);
        v = 1 - clamp01((e.clientY - rect.top) / rect.height);
    });

    pointerDrag(hueArea, (e) => {
        const rect = hueArea.getBoundingClientRect();
        h = clamp01((e.clientX - rect.left) / rect.width) * 360;
    });

    hexInput.addEventListener("change", () => {
        const parsed = parseColorLiteral(hexInput.value.trim());
        if (!parsed) {
            render();
            return;
        }
        ({ h, s, v } = hexToHsv(parsed.color));
        render();
        commit();
    });

    render();

    const anchorRect = anchor.getBoundingClientRect();
    const flyoutWidth = 292;
    el.style.left = Math.min(anchorRect.left, window.innerWidth - flyoutWidth - 8) + "px";
    el.style.top = anchorRect.bottom + 6 + "px";

    const onOutsidePointerDown = (e: PointerEvent) => {
        if (!el.contains(e.target as Node) && e.target !== anchor) closeFlyout();
    };
    const onKeyDown = (e: KeyboardEvent) => {
        if (e.key === "Escape") closeFlyout();
    };
    document.addEventListener("pointerdown", onOutsidePointerDown, true);
    document.addEventListener("keydown", onKeyDown, true);

    activeFlyout = { el, anchor, onOutsidePointerDown, onKeyDown };
}

class ColorSwatchWidget extends WidgetType {
    constructor(
        private readonly state: PickerState,
        private readonly color: string,
    ) {
        super();
    }

    eq(other: ColorSwatchWidget) {
        return (
            other.state.colorType === this.state.colorType &&
            other.color === this.color &&
            other.state.from === this.state.from &&
            other.state.to === this.state.to &&
            other.state.alpha === this.state.alpha
        );
    }

    toDOM(view: EditorView) {
        const swatch = document.createElement("span");
        swatch.className = "cm-color-swatch";
        swatch.style.background = this.color;
        swatch.addEventListener("mousedown", (e) => e.preventDefault());
        swatch.addEventListener("click", (e) => {
            e.preventDefault();
            openFlyout(view, swatch, this.state, this.color);
        });
        return swatch;
    }

    ignoreEvent() {
        return false;
    }
}

function colorPickerDecorations(view: EditorView): DecorationSet {
    const widgets: Range<Decoration>[] = [];
    const tree = syntaxTree(view.state);

    for (const range of view.visibleRanges) {
        tree.iterate({
            from: range.from,
            to: range.to,
            enter: ({ type, from, to }) => {
                const found = discoverColorsInCSS(tree, from, to, type.name, view.state.doc);
                if (!found) return;

                const list = Array.isArray(found) ? found : [found];
                for (const wo of list) {
                    widgets.push(
                        Decoration.widget({
                            widget: new ColorSwatchWidget(
                                { from: wo.from, to: wo.to, alpha: wo.alpha, colorType: wo.colorType },
                                wo.color,
                            ),
                            side: 1,
                        }).range(wo.from),
                    );
                }
            },
        });
    }

    return Decoration.set(widgets);
}

const colorPickerPlugin = ViewPlugin.fromClass(
    class {
        decorations: DecorationSet;

        constructor(view: EditorView) {
            this.decorations = colorPickerDecorations(view);
        }

        update(update: ViewUpdate) {
            if (update.docChanged || update.viewportChanged) {
                this.decorations = colorPickerDecorations(update.view);
            }
        }
    },
    { decorations: (v) => v.decorations },
);

const colorPickerBaseTheme = EditorView.baseTheme({
    ".cm-color-swatch": {
        display: "inline-block",
        width: "0.85em",
        height: "0.85em",
        marginRight: "0.5ch",
        borderRadius: "2px",
        outline: "1px solid var(--border)",
        cursor: "pointer",
        transform: "translateY(1px)",
    },
});

export const colorPicker = [colorPickerPlugin, colorPickerBaseTheme];
