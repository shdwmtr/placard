(function() {
    "use strict";

    var previewImg = document.getElementById("preview-img");
    var previewStatus = document.getElementById("preview-status");
    var previewViewport = document.getElementById("preview-viewport");
    var previewStage = document.getElementById("preview-stage");
    var zoomLevelEl = document.getElementById("zoom-level");
    var zoomResetBtn = document.getElementById("zoom-reset");
    var antialiasingToggle = document.getElementById("antialiasing-toggle");
    var autoWidthToggle = document.getElementById("auto-width-toggle");
    var widthValueInput = document.getElementById("width-value");
    var widthHint = document.getElementById("width-hint");
    var resizeHandle = document.getElementById("resize-handle");
    var resizePreview = document.getElementById("resize-preview");
    var formatSelect = document.getElementById("format");
    var copyBtn = document.getElementById("copy-url");
    var maximizeToggle = document.getElementById("maximize-toggle");
    var browserFrame = document.getElementById("browser-frame");
    var browserWidthReadout = document.getElementById("browser-width-readout");
    var browserPane = document.getElementById("browser-pane");
    var browserToggle = document.getElementById("browser-toggle");
    var consolePanel = document.getElementById("console-panel");
    var consoleToggle = document.getElementById("console-toggle");
    var consoleBody = document.getElementById("console-body");
    var consoleIcon = document.getElementById("console-icon");
    var consoleSummary = document.getElementById("console-summary");
    var sandboxWindow = document.getElementById("sandbox-window");
    var tabPanelsEl = document.querySelector(".tab-panels");
    var tabPanelsDivider = document.getElementById("tab-panels-divider");
    var editorPanel = document.querySelector('.tab-panel[data-panel="editor"]');
    var previewPanel = document.querySelector('.tab-panel[data-panel="preview"]');
    var tabButtons = Array.prototype.slice.call(
        document.querySelectorAll(".window-tab-btn"),
    );
    var tabPanels = Array.prototype.slice.call(
        document.querySelectorAll(".tab-panel"),
    );
    var layoutButtons = Array.prototype.slice.call(
        document.querySelectorAll(".layout-btn"),
    );

    function isPreviewVisible() {
        return (
            previewPanel.classList.contains("active") ||
            sandboxWindow.dataset.layout === "split"
        );
    }

    function activateTab(tab) {
        tabButtons.forEach(function(btn) {
            btn.classList.toggle("active", btn.dataset.tab === tab);
        });
        tabPanels.forEach(function(panel) {
            panel.classList.toggle("active", panel.dataset.panel === tab);
        });
        sandboxWindow.dataset.activeTab = tab;
        if (tab === "preview") {
            hasFitOnce = true;
            fitToViewport();
        }
    }

    tabButtons.forEach(function(btn) {
        btn.addEventListener("click", function() {
            activateTab(btn.dataset.tab);
        });
    });

    var LAYOUT_STORAGE_KEY = "placard-editor-layout";

    function setLayout(layout) {
        sandboxWindow.dataset.layout = layout;
        layoutButtons.forEach(function(btn) {
            btn.classList.toggle("active", btn.dataset.layout === layout);
        });
        try {
            localStorage.setItem(LAYOUT_STORAGE_KEY, layout);
        } catch (e) { }
        if (isPreviewVisible()) {
            hasFitOnce = true;
            fitToViewport();
        }
    }

    layoutButtons.forEach(function(btn) {
        btn.addEventListener("click", function() {
            setLayout(btn.dataset.layout);
        });
    });

    var storedLayout = null;
    try {
        storedLayout = localStorage.getItem(LAYOUT_STORAGE_KEY);
    } catch (e) { }
    if (storedLayout === "split" || storedLayout === "tabs") {
        sandboxWindow.dataset.layout = storedLayout;
        layoutButtons.forEach(function(btn) {
            btn.classList.toggle("active", btn.dataset.layout === storedLayout);
        });
    }

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

    function renderUrl(bypassCache) {
        var format = formatSelect.value || "webp";
        var params = [];
        if (bypassCache) {
            params.push("nocache=1");
        }
        if (!autoWidthToggle.checked && widthValueInput.value) {
            params.push("width=" + encodeURIComponent(widthValueInput.value));
        }
        if (!antialiasingToggle.checked) {
            params.push("antialiasing=0");
        }
        var query = params.length ? "?" + params.join("&") : "";
        return "/r/" + window.__placard.encode(editor.getValue()) + "." + format + query;
    }

    var currentObjectUrl = null;
    var previewToken = 0;
    var lastResolvedWidth = 400;

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
        items.forEach(function(item) {
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

    consoleToggle.addEventListener("click", function() {
        consolePanel.classList.toggle("expanded");
    });

    function updatePreview() {
        if (!editor) return;
        var myToken = ++previewToken;
        var url = renderUrl();
        previewStatus.textContent = "Rendering...";
        previewStatus.classList.remove("error");

        fetch(url, { cache: "no-store" })
            .then(function(res) {
                if (myToken !== previewToken) return;
                if (!res.ok) {
                    return res.text().then(function(text) {
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
                return res.blob().then(function(blob) {
                    if (myToken !== previewToken) return;
                    var objectUrl = URL.createObjectURL(blob);
                    previewImg.src = objectUrl;
                    if (currentObjectUrl) URL.revokeObjectURL(currentObjectUrl);
                    currentObjectUrl = objectUrl;
                    previewStatus.textContent = "";
                });
            })
            .catch(function() {
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
        var width =
            !autoWidthToggle.checked && widthValueInput.value
                ? widthValueInput.value
                : String(lastResolvedWidth);
        browserFrame.style.width = width + "px";
        browserWidthReadout.textContent = width + "px";
        browserFrame.srcdoc = editor.getValue();
    }

    function updateBrowserFrameHeight() {
        try {
            var doc = browserFrame.contentDocument;
            var height = Math.max(
                doc.documentElement.scrollHeight,
                doc.body ? doc.body.scrollHeight : 0,
            );
            browserFrame.style.height = height + "px";
        } catch (e) { }
    }

    browserFrame.addEventListener("load", updateBrowserFrameHeight);

    browserToggle.addEventListener("click", function() {
        var expanding = !browserPane.classList.contains("expanded");
        browserPane.classList.toggle("expanded");
        if (expanding) updateBrowserFrameHeight();
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

    previewImg.addEventListener("load", function() {
        if (previewImg.naturalWidth) {
            lastResolvedWidth = previewImg.naturalWidth;
            if (autoWidthToggle.checked) updateBrowserFrame();
        }
        if (!hasFitOnce && isPreviewVisible()) {
            hasFitOnce = true;
            fitToViewport();
        }
    });

    previewImg.addEventListener("dragstart", function(e) {
        e.preventDefault();
    });

    previewViewport.addEventListener(
        "wheel",
        function(e) {
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

    previewViewport.addEventListener("mousedown", function(e) {
        if (e.button !== 0) return;
        isDragging = true;
        dragStartX = e.clientX;
        dragStartY = e.clientY;
        dragOriginX = translateX;
        dragOriginY = translateY;
        previewViewport.classList.add("dragging");
    });

    window.addEventListener("mousemove", function(e) {
        if (!isDragging) return;
        translateX = dragOriginX + (e.clientX - dragStartX);
        translateY = dragOriginY + (e.clientY - dragStartY);
        applyTransform();
    });

    window.addEventListener("mouseup", function() {
        if (!isDragging) return;
        isDragging = false;
        previewViewport.classList.remove("dragging");
    });

    previewViewport.addEventListener("dblclick", fitToViewport);
    zoomResetBtn.addEventListener("click", fitToViewport);

    function setMaximized(on) {
        sandboxWindow.classList.toggle("maximized", on);
        document.body.classList.toggle("sandbox-lock-scroll", on);
        maximizeToggle.title = on ? "Restore" : "Maximize";
        maximizeToggle.setAttribute(
            "aria-label",
            on ? "Restore editor" : "Maximize editor",
        );
        if (isPreviewVisible()) {
            fitToViewport();
        }
    }

    maximizeToggle.addEventListener("click", function() {
        setMaximized(!sandboxWindow.classList.contains("maximized"));
    });

    window.addEventListener("keydown", function(e) {
        if (
            e.key === "Escape" &&
            sandboxWindow.classList.contains("maximized")
        ) {
            setMaximized(false);
        }
    });

    var MIN_RESIZE_WIDTH = 1;
    var MAX_RESIZE_WIDTH = 2000;
    var isResizing = false;
    var resizeStartX = 0;
    var resizeStartWidth = 400;

    function updateResizePreview(newWidth) {
        var left = Math.min(resizeStartWidth, newWidth);
        var width = Math.abs(newWidth - resizeStartWidth);
        resizePreview.style.left = left + "px";
        resizePreview.style.width = width + "px";
    }

    function setAutoWidth(auto) {
        widthValueInput.disabled = auto;
        widthValueInput.classList.toggle("hidden", auto);
        resizeHandle.classList.toggle("enabled", !auto);
        widthHint.classList.toggle("visible", !auto);
        if (auto) {
            widthValueInput.value = "";
        } else if (!widthValueInput.value) {
            widthValueInput.value = lastResolvedWidth;
        }
    }

    antialiasingToggle.addEventListener("change", updatePreview);

    autoWidthToggle.addEventListener("change", function() {
        setAutoWidth(autoWidthToggle.checked);
        updatePreview();
        updateBrowserFrame();
    });

    widthValueInput.addEventListener("change", function() {
        updatePreview();
        updateBrowserFrame();
    });

    resizeHandle.addEventListener("mousedown", function(e) {
        if (autoWidthToggle.checked || e.button !== 0) return;
        e.preventDefault();
        e.stopPropagation();
        isResizing = true;
        resizeStartX = e.clientX;
        resizeStartWidth = parseInt(widthValueInput.value, 10) || lastResolvedWidth;
        resizeHandle.classList.add("dragging");
        resizePreview.classList.add("active");
        updateResizePreview(resizeStartWidth);
    });

    window.addEventListener("mousemove", function(e) {
        if (!isResizing) return;
        var dx = (e.clientX - resizeStartX) / scale;
        var newWidth = Math.round(
            clamp(resizeStartWidth + dx, MIN_RESIZE_WIDTH, MAX_RESIZE_WIDTH),
        );
        widthValueInput.value = newWidth;
        updateResizePreview(newWidth);
    });

    window.addEventListener("mouseup", function() {
        if (!isResizing) return;
        isResizing = false;
        resizeHandle.classList.remove("dragging");
        resizePreview.classList.remove("active");
        updateBrowserFrame();
        updatePreview();
    });

    setAutoWidth(autoWidthToggle.checked);

    var MIN_SPLIT_RATIO = 0.15;
    var MAX_SPLIT_RATIO = 0.85;
    var isPanelDragging = false;

    function applySplitRatio(ratio) {
        editorPanel.style.flexBasis = ratio * 100 + "%";
        previewPanel.style.flexBasis = (1 - ratio) * 100 + "%";
        if (isPreviewVisible()) fitToViewport();
    }

    tabPanelsDivider.addEventListener("mousedown", function(e) {
        if (e.button !== 0) return;
        e.preventDefault();
        isPanelDragging = true;
        tabPanelsDivider.classList.add("dragging");
        document.body.style.cursor = "col-resize";
    });

    window.addEventListener("mousemove", function(e) {
        if (!isPanelDragging) return;
        var rect = tabPanelsEl.getBoundingClientRect();
        var ratio = clamp(
            (e.clientX - rect.left) / rect.width,
            MIN_SPLIT_RATIO,
            MAX_SPLIT_RATIO,
        );
        applySplitRatio(ratio);
    });

    window.addEventListener("mouseup", function() {
        if (!isPanelDragging) return;
        isPanelDragging = false;
        tabPanelsDivider.classList.remove("dragging");
        document.body.style.cursor = "";
    });

    tabPanelsDivider.addEventListener("dblclick", function() {
        editorPanel.style.flexBasis = "";
        previewPanel.style.flexBasis = "";
        if (isPreviewVisible()) fitToViewport();
    });

    var debounceTimer = null;
    function scheduleUpdate() {
        try {
            localStorage.setItem(STORAGE_KEY, editor.getValue());
        } catch (e) { }
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(function() {
            updatePreview();
            updateBrowserFrame();
        }, 1000);
    }

    formatSelect.addEventListener("change", updatePreview);

    copyBtn.addEventListener("click", function() {
        if (!editor) return;
        var url = window.location.origin + renderUrl();
        navigator.clipboard.writeText(url).then(function() {
            var original = copyBtn.textContent;
            copyBtn.textContent = "Copied!";
            setTimeout(function() {
                copyBtn.textContent = original;
            }, 1200);
        });
    });

    var cm = window.__cm;

    var initial = null;
    try {
        initial = localStorage.getItem(STORAGE_KEY);
    } catch (e) { }

    function isLightTheme() {
        return document.documentElement.getAttribute("data-theme") === "light";
    }

    var themeCompartment = new cm.Compartment();
    var highlightCompartment = new cm.Compartment();

    function editorTheme() {
        return isLightTheme() ? cm.placardThemeLight : cm.placardThemeDark;
    }

    function editorHighlightStyle() {
        return isLightTheme() ? cm.defaultHighlightStyle : cm.oneDarkHighlightStyle;
    }

    var view = new cm.EditorView({
        doc: initial || DEFAULT_SOURCE,
        parent: document.getElementById("editor-container"),
        extensions: [
            cm.basicSetup,
            cm.keymap.of([cm.indentWithTab]),
            cm.html(),
            cm.colorPicker,
            highlightCompartment.of(cm.syntaxHighlighting(editorHighlightStyle())),
            themeCompartment.of(editorTheme()),
            cm.indentUnit.of("\t"),
            cm.EditorState.tabSize.of(4),
            cm.EditorView.updateListener.of(function(update) {
                if (update.docChanged) scheduleUpdate();
            }),
        ],
    });

    window.addEventListener("placard-theme-change", function() {
        view.dispatch({
            effects: [
                themeCompartment.reconfigure(editorTheme()),
                highlightCompartment.reconfigure(
                    cm.syntaxHighlighting(editorHighlightStyle()),
                ),
            ],
        });
    });

    editor = {
        getValue: function() {
            return view.state.doc.toString();
        },
    };

    updatePreview();
    updateBrowserFrame();
})();
