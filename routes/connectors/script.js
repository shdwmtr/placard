(function () {
    "use strict";

    function el(tag, attrs, children) {
        var e = document.createElement(tag);
        if (attrs) {
            for (var k in attrs) {
                if (attrs[k] !== undefined && attrs[k] !== null) {
                    e.setAttribute(k, attrs[k]);
                }
            }
        }
        (children || []).forEach(function (c) {
            if (c === null || c === undefined) return;
            e.appendChild(
                typeof c === "string" ? document.createTextNode(c) : c,
            );
        });
        return e;
    }

    function tok(cls, text) {
        return el("span", { class: "tok-" + cls }, [text]);
    }

    function attrTok(name, value) {
        var frag = document.createDocumentFragment();
        frag.appendChild(tok("attr", "\n    data-" + name));
        frag.appendChild(tok("punct", "="));
        frag.appendChild(tok("string", '"' + value + '"'));
        return frag;
    }

    function exampleSnippet(preset) {
        var code = el("code");
        code.appendChild(tok("punct", "<"));
        code.appendChild(tok("tag", "span"));
        code.appendChild(attrTok("preset", preset.preset));
        preset.params.forEach(function (param) {
            code.appendChild(attrTok(param.name, param.example));
        });
        code.appendChild(document.createTextNode("\n"));
        code.appendChild(tok("punct", ">"));
        code.appendChild(document.createTextNode("\n    "));
        code.appendChild(tok("text", "0"));
        code.appendChild(document.createTextNode(" "));
        code.appendChild(
            tok("comment", "<!-- fallback text, kept if resolution fails -->"),
        );
        code.appendChild(document.createTextNode("\n"));
        code.appendChild(tok("punct", "</"));
        code.appendChild(tok("tag", "span"));
        code.appendChild(tok("punct", ">"));
        return el("pre", null, [code]);
    }

    function paramField(param) {
        if (param.options && param.options.length > 0) {
            var options = param.options.slice();
            if (options.indexOf(param.example) === -1) {
                options.unshift(param.example);
            }
            return el(
                "select",
                { "data-param": param.name },
                options.map(function (opt) {
                    var attrs = { value: opt };
                    if (opt === param.example) {
                        attrs.selected = "selected";
                    }
                    return el("option", attrs, [opt]);
                }),
            );
        }
        return el("input", {
            type: "text",
            "data-param": param.name,
            value: param.example,
            placeholder: param.name,
        });
    }

    function testerBlock(preset) {
        var fields = el("div", { class: "tester-fields" });
        preset.params.forEach(function (param) {
            fields.appendChild(
                el("label", null, [
                    el(
                        "span",
                        {
                            class:
                                "field-name " +
                                (param.required ? "req" : "opt"),
                        },
                        ["data-" + param.name],
                    ),
                    paramField(param),
                ]),
            );
        });
        if (preset.numeric) {
            fields.appendChild(
                el("label", null, [
                    el("span", { class: "field-name opt" }, [
                        "data-number-format",
                    ]),
                    el("input", {
                        type: "text",
                        "data-param": "number-format",
                        value: "",
                        placeholder: "%.2f",
                    }),
                ]),
            );
        }

        var bar = el("div", { class: "tester-bar" }, [
            el(
                "button",
                {
                    type: "button",
                    class: "run-btn",
                    "data-preset": preset.preset,
                },
                ["Run"],
            ),
        ]);

        var output = el("div", { class: "tester-output" }, [
            el("img", {
                class: "tester-img",
                alt: "rendered preview for " + preset.preset,
            }),
        ]);

        return el("div", { class: "tester" }, [fields, bar, output]);
    }

    function buildSidebarGroup(service, presets) {
        var ul = el("ul");
        presets.forEach(function (preset) {
            var a = el(
                "a",
                {
                    href: "#" + preset.preset,
                    "data-name": preset.preset,
                    "data-desc": preset.description,
                },
                [preset.preset],
            );
            ul.appendChild(el("li", null, [a]));
        });
        var summary = el("summary", null, [
            el("span", { class: "s-name" }, [service]),
            el("span", { class: "s-meta" }, [
                el("span", { class: "count" }, [String(presets.length)]),
                el("span", { class: "caret" }, ["›"]),
            ]),
        ]);
        return el("details", null, [summary, ul]);
    }

    function buildPresetSection(service, preset) {
        var main = el("div", { class: "preset-main" });
        main.appendChild(el("div", { class: "crumb" }, [service + "/"]));
        main.appendChild(el("h1", null, [preset.preset]));
        if (preset.description) {
            main.appendChild(el("p", { class: "desc" }, [preset.description]));
        } else {
            main.appendChild(
                el("p", { class: "desc placeholder" }, [
                    "No description mined yet.",
                ]),
            );
        }

        main.appendChild(el("hr"));
        main.appendChild(
            el("h2", { id: "params-" + preset.preset }, ["Parameters"]),
        );

        if (preset.params.length === 0 && !preset.numeric) {
            main.appendChild(el("p", { class: "dim" }, ["No parameters."]));
        } else {
            var tbody = el("tbody");
            preset.params.forEach(function (param) {
                var options = param.options || [];
                tbody.appendChild(
                    el("tr", null, [
                        el("td", { class: "mono" }, ["data-" + param.name]),
                        el("td", null, [
                            param.required ? "required" : "optional",
                        ]),
                        el("td", { class: "mono" }, [param.example]),
                        el(
                            "td",
                            { class: "mono" },
                            options.length > 0
                                ? [options.join(" · ")]
                                : [el("span", { class: "dim" }, ["--"])],
                        ),
                    ]),
                );
            });
            if (preset.numeric) {
                tbody.appendChild(
                    el(
                        "tr",
                        {
                            title:
                                "Reformats this preset's numeric result: printf-style precision/grouping, " +
                                "or a K/M/B/T scale suffix. Not part of the resolver itself -- ignored if " +
                                "the spec is malformed or the value isn't a plain number.",
                        },
                        [
                            el("td", { class: "mono" }, ["data-number-format"]),
                            el("td", { class: "universal" }, [
                                "optional · numeric",
                            ]),
                            el("td", { class: "mono" }, ["%.2f · %,d · %.0fK"]),
                            el("td", { class: "mono" }, [
                                el("span", { class: "dim" }, ["--"]),
                            ]),
                        ],
                    ),
                );
            }
            main.appendChild(
                el("table", null, [
                    el("thead", null, [
                        el("tr", null, [
                            el("th", null, ["Attribute"]),
                            el("th", null, ["Required"]),
                            el("th", null, ["Example"]),
                            el("th", null, ["Options"]),
                        ]),
                    ]),
                    tbody,
                ]),
            );
        }

        main.appendChild(exampleSnippet(preset));

        main.appendChild(el("hr"));
        main.appendChild(
            el("h2", { id: "tester-" + preset.preset }, ["Try it"]),
        );
        main.appendChild(testerBlock(preset));

        var toc = el("aside", { class: "preset-toc" }, [
            el("div", { class: "toc-label" }, ["On this page"]),
        ]);
        if (preset.params.length > 0) {
            toc.appendChild(
                el(
                    "a",
                    {
                        class: "toc-link",
                        "data-scrollto": "params-" + preset.preset,
                    },
                    ["Parameters"],
                ),
            );
        }
        toc.appendChild(
            el(
                "a",
                {
                    class: "toc-link",
                    "data-scrollto": "tester-" + preset.preset,
                },
                ["Try it"],
            ),
        );

        return el("section", { class: "preset", id: preset.preset }, [
            main,
            toc,
        ]);
    }

    function renderPresets(presets) {
        var byService = {};
        presets.forEach(function (p) {
            (byService[p.service] = byService[p.service] || []).push(p);
        });
        var services = Object.keys(byService).sort();
        services.forEach(function (service) {
            byService[service].sort(function (a, b) {
                return a.preset < b.preset ? -1 : a.preset > b.preset ? 1 : 0;
            });
        });

        var sidebar = document.getElementById("sidebar");
        var content = document.getElementById("content");

        services.forEach(function (service) {
            sidebar.appendChild(buildSidebarGroup(service, byService[service]));
            byService[service].forEach(function (preset) {
                content.appendChild(buildPresetSection(service, preset));
            });
        });
    }

    // Everything below is interactive behavior that only needs the DOM
    // built above -- sidebar search/filter, hash-routing, and the
    // tester's run button, which just calls /r/ directly like the sandbox
    // editor does.
    function initInteractions() {
        var search = document.getElementById("search");
        var sidebar = document.getElementById("sidebar");
        var links = Array.prototype.slice.call(sidebar.querySelectorAll("a"));

        search.addEventListener("input", function () {
            var q = search.value.trim().toLowerCase();
            links.forEach(function (a) {
                var li = a.parentElement;
                var haystack = (
                    a.dataset.name +
                    " " +
                    a.dataset.desc
                ).toLowerCase();
                var match = q === "" || haystack.indexOf(q) !== -1;
                li.classList.toggle("hidden", !match);
            });
            Array.prototype.slice
                .call(sidebar.querySelectorAll("details"))
                .forEach(function (d) {
                    var anyVisible = Array.prototype.slice
                        .call(d.querySelectorAll("li"))
                        .some(function (li) {
                            return !li.classList.contains("hidden");
                        });
                    d.classList.toggle("hidden", !anyVisible);
                    if (q !== "" && anyVisible) {
                        d.open = true;
                    }
                });
        });

        document.addEventListener("keydown", function (e) {
            if (e.key === "/" && document.activeElement !== search) {
                e.preventDefault();
                search.focus();
            }
        });

        function highlightActive() {
            var current = decodeURIComponent(location.hash.slice(1));
            links.forEach(function (a) {
                a.classList.toggle("active", a.dataset.name === current);
            });
        }
        window.addEventListener("hashchange", highlightActive);
        highlightActive();

        if (!location.hash && links.length > 0) {
            location.replace(
                location.pathname +
                    location.search +
                    links[0].getAttribute("href"),
            );
        }

        function base64url(str) {
            var bytes = new TextEncoder().encode(str);
            var binary = "";
            for (var i = 0; i < bytes.length; i++) {
                binary += String.fromCharCode(bytes[i]);
            }
            return btoa(binary)
                .replace(/\+/g, "-")
                .replace(/\//g, "_")
                .replace(/=+$/, "");
        }

        function attrEscape(value) {
            return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
        }

        document
            .getElementById("content")
            .addEventListener("click", function (e) {
                var tocLink = e.target.closest(".toc-link");
                if (tocLink) {
                    var el = document.getElementById(tocLink.dataset.scrollto);
                    if (el) {
                        el.scrollIntoView({
                            behavior: "smooth",
                            block: "start",
                        });
                    }
                    return;
                }

                var btn = e.target.closest(".run-btn");
                if (!btn) {
                    return;
                }
                var section = btn.closest(".preset");
                var inputs = Array.prototype.slice.call(
                    section.querySelectorAll(".tester input[data-param]"),
                );
                var html = '<span data-preset="' + btn.dataset.preset + '"';
                inputs.forEach(function (input) {
                    var value = input.value.trim();
                    if (value !== "") {
                        html +=
                            " data-" +
                            input.dataset.param +
                            '="' +
                            attrEscape(value) +
                            '"';
                    }
                });
                html += ">0</span>";

                var img = section.querySelector(".tester-img");
                img.removeAttribute("src");
                img.alt = "rendering...";
                img.src = "/r/" + base64url(html) + ".webp";
                img.onload = function () {
                    img.alt = "rendered preview for " + btn.dataset.preset;
                };
                img.onerror = function () {
                    img.alt =
                        "render failed -- check the parameter values above";
                };
            });
    }

    fetch("/presets")
        .then(function (res) {
            return res.json();
        })
        .then(function (presets) {
            renderPresets(presets);
            initInteractions();
        })
        .catch(function () {
            document.getElementById("content").textContent =
                "Failed to load presets -- is the server running?";
        });
})();
