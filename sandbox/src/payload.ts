import { deflateRaw } from "pako";
import { DICTIONARY_B64 } from "./dictionary";

const FRAME_TAG = 0x00;
const SCHEME_RAW = 0x01;
const SCHEME_DICT = 0x02;

function base64ToBytes(b64: string): Uint8Array {
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
}

function bytesToBase64Url(bytes: Uint8Array): string {
    let binary = "";
    for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
    return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

const dictionary = base64ToBytes(DICTIONARY_B64);
const noDictionary = new Uint8Array(0);

export function encode(html: string): string {
    const bytes = new TextEncoder().encode(html);
    let best = bytesToBase64Url(bytes);

    const candidates: Array<[number, Uint8Array]> = [
        [SCHEME_RAW, noDictionary],
        [SCHEME_DICT, dictionary],
    ];
    for (const [scheme, dict] of candidates) {
        // Always pass an explicit dictionary (even an empty one) rather
        // than omitting the option -- Bun's minifier corrupts pako's
        // internal `new Uint8Array(0)` default value for an omitted
        // dictionary, so relying on it silently breaks in the built bundle
        // even though it works fine unminified.
        const compressed = deflateRaw(bytes, { level: 9, dictionary: dict });
        const framed = new Uint8Array(compressed.length + 2);
        framed[0] = FRAME_TAG;
        framed[1] = scheme;
        framed.set(compressed, 2);

        const candidate = bytesToBase64Url(framed);
        if (candidate.length < best.length) best = candidate;
    }

    return best;
}
