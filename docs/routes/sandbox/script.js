(function () {
    "use strict";

    var previewImg = document.getElementById("preview-img");
    var previewStatus = document.getElementById("preview-status");
    var previewViewport = document.getElementById("preview-viewport");
    var previewStage = document.getElementById("preview-stage");
    var zoomLevelEl = document.getElementById("zoom-level");
    var zoomResetBtn = document.getElementById("zoom-reset");
    var borderWidthInput = document.getElementById("border-width");
    var borderWidthValueEl = document.getElementById("border-width-value");
    var borderColorInput = document.getElementById("border-color");
    var widthInput = document.getElementById("width");
    var formatSelect = document.getElementById("format");
    var copyBtn = document.getElementById("copy-url");
    var browserFrame = document.getElementById("browser-frame");
    var browserWidthReadout = document.getElementById("browser-width-readout");
    var consolePanel = document.getElementById("console-panel");
    var consoleToggle = document.getElementById("console-toggle");
    var consoleBody = document.getElementById("console-body");
    var consoleIcon = document.getElementById("console-icon");
    var consoleSummary = document.getElementById("console-summary");

    var STORAGE_KEY = "placard-editor-source";
    var DEFAULT_SOURCE = [
        "<body>",
        '\t<div class="wrap">',
        '\t\t<span class="label">build</span>',
        '\t\t<span class="value">passing</span>',
        "\t</div>",
        "</body>",
        "<style>",
        "\tbody { margin: 0; background: #000000; }",
        "\t.wrap { display: flex; font-family: monospace; font-size: 14px; font-weight: bold; }",
        "\t.label { background: #1a1a1a; color: #a1a1a1; padding: 8px 14px; border-radius: 6px 0 0 6px; }",
        "\t.value { background: #2ea043; color: #ffffff; padding: 8px 14px; border-radius: 0 6px 6px 0; }",
        "</style>",
        "",
    ].join("\n");

    var editor = null;

    function base64url(str) {
        var bytes = new TextEncoder().encode(str);
        var binary = "";
        for (var i = 0; i < bytes.length; i++)
            binary += String.fromCharCode(bytes[i]);
        return btoa(binary)
            .replace(/\+/g, "-")
            .replace(/\//g, "_")
            .replace(/=+$/, "");
    }

    function renderUrl() {
        var width = widthInput.value || "400";
        var format = formatSelect.value || "webp";
        return (
            "/r/" +
            base64url(editor.getValue()) +
            "." +
            format +
            "?width=" +
            encodeURIComponent(width)
        );
    }

    var currentObjectUrl = null;
    var previewToken = 0;

    function renderConsole(items) {
        consoleBody.textContent = "";
        consolePanel.classList.remove("has-errors", "has-warnings-only");

        if (!items.length) {
            consolePanel.classList.remove("visible", "expanded");
            consoleSummary.textContent = "";
            consoleIcon.textContent = "";
            return;
        }

        var errorCount = 0;
        var warningCount = 0;
        items.forEach(function (item) {
            var isError = item.severity === "error";
            if (isError) {
                errorCount++;
            } else {
                warningCount++;
            }
            var line = document.createElement("div");
            line.className = "console-line " + (isError ? "error" : "warning");
            line.textContent = item.message;
            consoleBody.appendChild(line);
        });

        var parts = [];
        if (errorCount)
            parts.push(errorCount + (errorCount === 1 ? " error" : " errors"));
        if (warningCount)
            parts.push(
                warningCount + (warningCount === 1 ? " warning" : " warnings"),
            );
        consoleSummary.textContent = parts.join(", ");
        consoleIcon.textContent = errorCount > 0 ? "✕" : "⚠";

        consolePanel.classList.add("visible", "expanded");
        consolePanel.classList.add(
            errorCount > 0 ? "has-errors" : "has-warnings-only",
        );
    }

    function parseDiagnosticsHeader(res) {
        var raw = res.headers.get("X-Placard-Diagnostics");
        if (!raw) return [];
        try {
            var parsed = JSON.parse(raw);
            return Array.isArray(parsed) ? parsed : [];
        } catch (e) {
            return [];
        }
    }

    consoleToggle.addEventListener("click", function () {
        consolePanel.classList.toggle("expanded");
    });

    function updatePreview() {
        if (!editor) return;
        var myToken = ++previewToken;
        var url = renderUrl();
        previewStatus.textContent = "Rendering...";
        previewStatus.classList.remove("error");

        fetch(url)
            .then(function (res) {
                if (myToken !== previewToken) return;
                if (!res.ok) {
                    return res.text().then(function (text) {
                        if (myToken !== previewToken) return;
                        var message =
                            text || "Render failed (" + res.status + ")";
                        previewStatus.textContent = message;
                        previewStatus.classList.add("error");
                        renderConsole([
                            { severity: "error", message: message },
                        ]);
                    });
                }
                renderConsole(parseDiagnosticsHeader(res));
                return res.blob().then(function (blob) {
                    if (myToken !== previewToken) return;
                    var objectUrl = URL.createObjectURL(blob);
                    previewImg.src = objectUrl;
                    if (currentObjectUrl) URL.revokeObjectURL(currentObjectUrl);
                    currentObjectUrl = objectUrl;
                    previewStatus.textContent = "";
                });
            })
            .catch(function () {
                if (myToken !== previewToken) return;
                previewStatus.textContent = "Network error";
                previewStatus.classList.add("error");
                renderConsole([
                    { severity: "error", message: "Network error" },
                ]);
            });
    }

    function updateBrowserFrame() {
        if (!editor) return;
        var width = widthInput.value || "400";
        browserFrame.style.width = width + "px";
        browserWidthReadout.textContent = width + "px";
        browserFrame.srcdoc = editor.getValue();
    }

    browserFrame.addEventListener("load", function () {
        try {
            var doc = browserFrame.contentDocument;
            var height = Math.max(
                doc.documentElement.scrollHeight,
                doc.body ? doc.body.scrollHeight : 0,
            );
            browserFrame.style.height = height + "px";
        } catch (e) {}
    });

    var MIN_SCALE = 0.1;
    var MAX_SCALE = 10;
    var scale = 1;
    var translateX = 0;
    var translateY = 0;
    var hasFitOnce = false;

    function clamp(value, min, max) {
        return Math.min(max, Math.max(min, value));
    }

    function applyTransform() {
        previewStage.style.transform =
            "translate(" +
            translateX +
            "px, " +
            translateY +
            "px) scale(" +
            scale +
            ")";
        zoomLevelEl.textContent = Math.round(scale * 100) + "%";
    }

    function fitToViewport() {
        var iw = previewImg.naturalWidth || 1;
        var ih = previewImg.naturalHeight || 1;
        var vw = previewViewport.clientWidth;
        var vh = previewViewport.clientHeight;
        scale = clamp(Math.min(vw / iw, vh / ih, 1), MIN_SCALE, MAX_SCALE);
        translateX = (vw - iw * scale) / 2;
        translateY = (vh - ih * scale) / 2;
        applyTransform();
    }

    previewImg.addEventListener("load", function () {
        if (!hasFitOnce) {
            hasFitOnce = true;
            fitToViewport();
        }
    });

    previewImg.addEventListener("dragstart", function (e) {
        e.preventDefault();
    });

    previewViewport.addEventListener(
        "wheel",
        function (e) {
            e.preventDefault();
            var rect = previewViewport.getBoundingClientRect();
            var cx = e.clientX - rect.left;
            var cy = e.clientY - rect.top;
            var zoomFactor = Math.exp(-e.deltaY * 0.001);
            var newScale = clamp(scale * zoomFactor, MIN_SCALE, MAX_SCALE);
            translateX = cx - (cx - translateX) * (newScale / scale);
            translateY = cy - (cy - translateY) * (newScale / scale);
            scale = newScale;
            applyTransform();
        },
        { passive: false },
    );

    var isDragging = false;
    var dragStartX = 0;
    var dragStartY = 0;
    var dragOriginX = 0;
    var dragOriginY = 0;

    previewViewport.addEventListener("mousedown", function (e) {
        if (e.button !== 0) return;
        isDragging = true;
        dragStartX = e.clientX;
        dragStartY = e.clientY;
        dragOriginX = translateX;
        dragOriginY = translateY;
        previewViewport.classList.add("dragging");
    });

    window.addEventListener("mousemove", function (e) {
        if (!isDragging) return;
        translateX = dragOriginX + (e.clientX - dragStartX);
        translateY = dragOriginY + (e.clientY - dragStartY);
        applyTransform();
    });

    window.addEventListener("mouseup", function () {
        if (!isDragging) return;
        isDragging = false;
        previewViewport.classList.remove("dragging");
    });

    previewViewport.addEventListener("dblclick", fitToViewport);
    zoomResetBtn.addEventListener("click", fitToViewport);

    function updateBorder() {
        previewImg.style.borderWidth = borderWidthInput.value + "px";
        previewImg.style.borderColor = borderColorInput.value;
        borderWidthValueEl.textContent = borderWidthInput.value + "px";
    }

    borderWidthInput.addEventListener("input", updateBorder);
    borderColorInput.addEventListener("input", updateBorder);
    updateBorder();

    var debounceTimer = null;
    function scheduleUpdate() {
        try {
            localStorage.setItem(STORAGE_KEY, editor.getValue());
        } catch (e) {}
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(function () {
            updatePreview();
            updateBrowserFrame();
        }, 350);
    }

    widthInput.addEventListener("change", function () {
        updatePreview();
        updateBrowserFrame();
    });
    formatSelect.addEventListener("change", updatePreview);

    copyBtn.addEventListener("click", function () {
        if (!editor) return;
        var url = window.location.origin + renderUrl();
        navigator.clipboard.writeText(url).then(function () {
            var original = copyBtn.textContent;
            copyBtn.textContent = "Copied!";
            setTimeout(function () {
                copyBtn.textContent = original;
            }, 1200);
        });
    });

    var cm = window.__cm;

    var initial = null;
    try {
        initial = localStorage.getItem(STORAGE_KEY);
    } catch (e) {}

    var view = new cm.EditorView({
        doc: initial || DEFAULT_SOURCE,
        parent: document.getElementById("editor-container"),
        extensions: [
            cm.basicSetup,
            cm.keymap.of([cm.indentWithTab]),
            cm.html(),
            cm.syntaxHighlighting(cm.oneDarkHighlightStyle),
            cm.placardTheme,
            cm.indentUnit.of("\t"),
            cm.EditorState.tabSize.of(4),
            cm.EditorView.updateListener.of(function (update) {
                if (update.docChanged) scheduleUpdate();
            }),
        ],
    });

    editor = {
        getValue: function () {
            return view.state.doc.toString();
        },
    };

    updatePreview();
    updateBrowserFrame();
})();
