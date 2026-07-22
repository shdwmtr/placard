import { EditorView, keymap } from "@codemirror/view";
import { EditorState, Compartment } from "@codemirror/state";
import { basicSetup } from "codemirror";
import { html } from "@codemirror/lang-html";
import { syntaxHighlighting, defaultHighlightStyle, indentUnit } from "@codemirror/language";
import { indentWithTab } from "@codemirror/commands";
import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import { colorPicker } from "./color-picker";
import { encode as encodePayload } from "./payload";

const selectionColor = "#3b82f659";

const placardThemeDark = EditorView.theme(
    {
        "&": { color: "#ededed", backgroundColor: "#000000", height: "100%" },
        ".cm-content": { caretColor: "#ededed" },
        ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#ededed" },
        "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
            backgroundColor: selectionColor,
        },
        ".cm-gutters": { backgroundColor: "#000000", color: "#4a4a4a", border: "none" },
        ".cm-activeLineGutter": { backgroundColor: "#0a0a0a", color: "#a1a1a1" },
        ".cm-activeLine": { backgroundColor: "#ffffff0a" },
        ".cm-scroller": {
            fontFamily: '"Geist Mono", ui-monospace, "JetBrains Mono", monospace',
            fontSize: "13px",
            lineHeight: "21px",
        },
    },
    { dark: true },
);

const placardThemeLight = EditorView.theme(
    {
        "&": { color: "#000000", backgroundColor: "#ffffff", height: "100%" },
        ".cm-content": { caretColor: "#000000" },
        ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#000000" },
        "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
            backgroundColor: selectionColor,
        },
        ".cm-gutters": { backgroundColor: "#ffffff", color: "#8a8a8a", border: "none" },
        ".cm-activeLineGutter": { backgroundColor: "#f5f5f5", color: "#5e5e5e" },
        ".cm-activeLine": { backgroundColor: "#0000000a" },
        ".cm-scroller": {
            fontFamily: '"Geist Mono", ui-monospace, "JetBrains Mono", monospace',
            fontSize: "13px",
            lineHeight: "21px",
        },
    },
    { dark: false },
);

const cm = {
    EditorView,
    EditorState,
    Compartment,
    basicSetup,
    html,
    syntaxHighlighting,
    oneDarkHighlightStyle,
    defaultHighlightStyle,
    placardThemeDark,
    placardThemeLight,
    indentUnit,
    keymap,
    indentWithTab,
    colorPicker,
};

const placard = {
    encode: encodePayload,
};

declare global {
    interface Window {
        __cm: typeof cm;
        __placard: typeof placard;
    }
}

window.__cm = cm;
window.__placard = placard;
