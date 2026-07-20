(function () {
    "use strict";

    var STORAGE_KEY = "placard-theme";

    function getStoredTheme() {
        try {
            return localStorage.getItem(STORAGE_KEY);
        } catch (e) {
            return null;
        }
    }

    function applyTheme(theme) {
        document.documentElement.setAttribute("data-theme", theme);
    }

    applyTheme(getStoredTheme() || "dark");

    function setTheme(theme) {
        applyTheme(theme);
        try {
            localStorage.setItem(STORAGE_KEY, theme);
        } catch (e) {}
        window.dispatchEvent(
            new CustomEvent("placard-theme-change", { detail: { theme: theme } }),
        );
    }

    window.placardTheme = {
        get: function () {
            return document.documentElement.getAttribute("data-theme") || "dark";
        },
        set: setTheme,
        toggle: function () {
            setTheme(window.placardTheme.get() === "light" ? "dark" : "light");
        },
    };

    document.addEventListener("DOMContentLoaded", function () {
        var toggles = document.querySelectorAll("[data-theme-toggle]");
        for (var i = 0; i < toggles.length; i++) {
            toggles[i].addEventListener("click", window.placardTheme.toggle);
        }
    });
})();
