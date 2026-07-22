/// Corpus behind the preset DEFLATE dictionary for scheme `0x02`. Two
/// sections: a generic shields.io-style "build passing" pill (the one
/// truly universal badge convention across the ecosystem), followed by
/// generic CSS/HTML idioms plus real `data-preset`/param attribute shapes
/// pulled from `placard-connectors`' most commonly used services. Neither
/// example HTML file in the repo belongs here -- both are one-off,
/// project-specific content, not representative of an arbitrary badge.
const DICTIONARY_SOURCE: &str = "\
<body>
\t<div class=\"wrap\">
\t\t<span class=\"label\">build</span>
\t\t<span class=\"value\">passing</span>
\t</div>
</body>
<style>
\tbody { margin: 0; background: #000000; }
\t.wrap { display: flex; font-family: monospace; font-size: 14px; font-weight: bold; }
\t.label { background: #1a1a1a; color: #a1a1a1; padding: 8px 14px; border-radius: 6px 0 0 6px; }
\t.value { background: #2ea043; color: #ffffff; padding: 8px 14px; border-radius: 0 6px 6px 0; }
</style>
<span data-preset=\"github-stars\" data-owner=\"\" data-repo=\"\"></span>
<span data-preset=\"npm-version\" data-package=\"\"></span>
<span data-preset=\"crates-downloads\" data-crate=\"\"></span>
<span data-preset=\"pypi-downloads\" data-package=\"\"></span>
<span data-preset=\"docker-pulls\" data-user=\"\" data-repo=\"\"></span>
<html><head></head><body></body></html>
<div class=\"item\"><span class=\"label\"></span><span class=\"value\"></span></div>
<div style=\"display: flex\"><div class=\"item detail\"></div><div class=\"item status\"></div></div>
data-preset=\"\" data-connector=\"\" data-number-format=\"\" data-owner=\"\" data-repo=\"\" data-package=\"\" data-user=\"\" data-branch=\"\" data-project=\"\"
font-family: \"Geist Mono\", ui-monospace, \"JetBrains Mono\", monospace;
font-family: \"JetBrainsMono Nerd Font Mono\";
font-family: \"Geist\";
text-align: center; justify-content: center; align-items: center;
border-radius: 6px 0 0 6px; border-radius: 0 6px 6px 0; border-radius: 4px;
border: 1px solid #262626; border: 1px solid #1a1a1a;
font-weight: 700; font-weight: 600; font-weight: bold; font-size: 14px; font-size: 12px;
line-height: 1; letter-spacing: -0.05em;
background: #000000; background: #ffffff; background: #1a1a1a; background: #0a0a0a;
color: #ffffff; color: #000000; color: #a1a1a1; color: #757575; color: white; color: black;
margin: 0; padding: 0; padding: 8px 14px; padding: 6px 12px; padding: 10px;
display: flex; display: block; display: inline-block;
width: 100%; height: 100%;
</span></div></body></html>
";

/// The preset DEFLATE dictionary behind scheme `0x02`. These bytes are
/// frozen forever once that scheme ships -- decoding an already-issued
/// scheme-`0x02` URL must keep working exactly as it does today. A future
/// improved dictionary ships as a new scheme id (e.g. `DICTIONARY_V2` behind
/// `0x03`), it never replaces this one.
///
/// Exposed at the crate root so `examples/gen_dictionary` can derive
/// `sandbox/src/dictionary.ts`'s base64 blob from these exact bytes instead
/// of maintaining a second, hand-synced copy.
pub static DICTIONARY_V1: &[u8] = DICTIONARY_SOURCE.as_bytes();
